import { object, parseBackends, parseEvent, parseHistory, parseSession, parseSnapshot } from './model';
import type { Decision, HistoryPage, Session, Snapshot, WorkEvent } from './model';

const MAX_BYTES = 32 * 1024 * 1024;
export class ApiError extends Error {
  constructor(public readonly status: number) {
    super(({ 0: 'Kernel unreachable. Check that laind is running and the same-origin connection is available.',
      401: 'Credentials invalid. Reconnect with the Kernel API token.',
      400: 'Kernel rejected the input. Check the daemon workspace path and selected options.',
      404: 'This session or request no longer exists.',
      409: 'State changed before the action was accepted. Review the current session and try again.',
      422: 'Kernel rejected the input format.',
      502: 'Worker operation failed. Inspect the normalized timeline; no completion is assumed.',
      503: 'Kernel not ready or backend unavailable. Check Kernel and backend status.' } as Record<number, string>)[status]
      ?? `Kernel returned HTTP ${status}.`);
  }
}
export function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : 'Connection interrupted or invalid Kernel response. Reconnect to recover current state.';
}
export class SseDecoder {
  private pending = '';
  private name = '';
  private data: string[] = [];
  private bytes = 0;
  private decoder = new TextDecoder('utf-8', { fatal: true });
  push(chunk: Uint8Array): { name: string; data: string }[] {
    this.pending += this.decoder.decode(chunk, { stream: true });
    const frames: { name: string; data: string }[] = [];
    let end: number;
    while ((end = this.pending.indexOf('\n')) >= 0) {
      const raw = this.pending.slice(0, end);
      this.pending = this.pending.slice(end + 1);
      this.bytes += new TextEncoder().encode(raw).length + 1;
      if (this.bytes > MAX_BYTES) throw new Error('SSE frame too large');
      const line = raw.replace(/\r$/, '');
      if (!line) {
        if (this.data.length) frames.push({ name: this.name || 'message', data: this.data.join('\n') });
        this.name = ''; this.data = []; this.bytes = 0;
      } else if (line.startsWith('event:')) this.name = line.slice(6).replace(/^ /, '');
      else if (line.startsWith('data:')) this.data.push(line.slice(5).replace(/^ /, ''));
    }
    if (this.bytes + new TextEncoder().encode(this.pending).length > MAX_BYTES) throw new Error('SSE frame too large');
    return frames;
  }
}
export type StreamUpdate = { kind: 'snapshot'; snapshot: Snapshot } | { kind: 'event'; event: WorkEvent };
export class Client {
  #token: string;
  constructor(token: string) {
    if (!/^[A-Za-z\d_-]{32,256}$/.test(token)) throw new ApiError(401);
    this.#token = token;
  }
  private async fetch(path: string, signal: AbortSignal, body?: unknown): Promise<Response> {
    try {
      const response = await fetch(path, { method: body === undefined ? 'GET' : 'POST',
        headers: { Authorization: `Bearer ${this.#token}`, ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
        body: body === undefined ? undefined : JSON.stringify(body), signal, cache: 'no-store',
        credentials: 'omit', redirect: 'error' });
      if (!response.ok) { await response.body?.cancel(); throw new ApiError(response.status); }
      return response;
    } catch (error) {
      if (error instanceof ApiError || signal.aborted) throw error;
      throw new ApiError(0);
    }
  }
  async request(path: string, signal?: AbortSignal, body?: unknown): Promise<unknown> {
    const response = await this.fetch(path, AbortSignal.any([AbortSignal.timeout(60_000), ...(signal ? [signal] : [])]), body);
    const reader = response.body?.getReader();
    if (!reader) throw new Error('Empty response');
    const chunks: Uint8Array[] = []; let length = 0;
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        length += value.length;
        if (length > MAX_BYTES) throw new Error('API response too large');
        chunks.push(value);
      }
      const bytes = new Uint8Array(length); let offset = 0;
      for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
      return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
  }
  async status(signal?: AbortSignal) {
    const [ready, backends] = await Promise.all([
      this.request('/ready', signal).then(v => String(object(v).state)).catch(error => {
        if (error instanceof ApiError && error.status === 503) return 'not_ready';
        throw error;
      }),
      this.request('/v1/backends', signal).then(parseBackends),
    ]);
    return { ready, backends };
  }
  async create(workspace: string, model: string, sandbox: string): Promise<Session> {
    return parseSession(await this.request('/v1/work-sessions', undefined, { workspace,
      options: { model: model || null, sandbox } }));
  }
  prompt(session: string, text: string) { return this.request(`/v1/work-sessions/${session}/turns`, undefined, { text }); }
  cancel(session: string) { return this.request(`/v1/work-sessions/${session}/cancel`, undefined, {}); }
  resume(session: string) { return this.request(`/v1/work-sessions/${session}/resume`, undefined, {}); }
  decide(session: string, approval: string, decision: Decision) {
    return this.request(`/v1/work-sessions/${session}/approvals/${approval}/decision`, undefined, { decision });
  }
  async history(session: string, query: Record<string, number>, signal: AbortSignal): Promise<HistoryPage> {
    const params = new URLSearchParams(Object.entries({ limit: 200, ...query }).map(([key, value]) => [key, String(value)]));
    const page = parseHistory(await this.request(`/v1/work-sessions/${session}/history?${params}`, signal));
    if (page.events.some(event => event.work_session_id !== session)) throw new Error('Foreign session history');
    return page;
  }
  async stream(signal: AbortSignal, update: (value: StreamUpdate) => void): Promise<never> {
    const controller = new AbortController();
    const combined = AbortSignal.any([signal, controller.signal]);
    let timer = setTimeout(() => controller.abort(), 45_000);
    let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
    try {
      const response = await this.fetch('/v1/events', combined);
      if (!response.headers.get('content-type')?.startsWith('text/event-stream')) throw new Error('Not SSE');
      reader = response.body?.getReader();
      if (!reader) throw new Error('Missing stream');
      const decoder = new SseDecoder(); let snapshot = false;
      for (;;) {
        const { value, done } = await reader.read();
        if (done) throw new Error('SSE closed');
        clearTimeout(timer); timer = setTimeout(() => controller.abort(), 45_000);
        for (const frame of decoder.push(value)) {
          if (frame.name === 'resync_required') throw new Error('Resync requested');
          if (frame.name === 'snapshot') {
            snapshot = true; update({ kind: 'snapshot', snapshot: parseSnapshot(JSON.parse(frame.data)) });
          } else if (frame.name === 'event') {
            if (!snapshot) throw new Error('Event before snapshot');
            update({ kind: 'event', event: parseEvent(JSON.parse(frame.data)) });
          }
        }
      }
    } finally {
      clearTimeout(timer); controller.abort();
      await reader?.cancel().catch(() => {}); reader?.releaseLock();
    }
  }
}

export async function observe(client: Client, signal: AbortSignal, update: (value: StreamUpdate) => void,
  offline: (error: unknown) => void, delay = 2000): Promise<void> {
  while (!signal.aborted) {
    try { await client.stream(signal, update); }
    catch (error) {
      if (signal.aborted) return;
      offline(error);
      if (error instanceof ApiError && error.status === 401) return;
    }
    await new Promise<void>(resolve => {
      const abort = () => { clearTimeout(timer); resolve(); };
      const timer = setTimeout(() => { signal.removeEventListener('abort', abort); resolve(); }, delay);
      signal.addEventListener('abort', abort, { once: true });
      if (signal.aborted) abort();
    });
  }
}
