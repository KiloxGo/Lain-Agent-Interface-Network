import { afterEach, describe, expect, it, vi } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ApiError, Client, observe, SseDecoder } from '../src/api';
import { ApprovalCard, StateBadges } from '../src/App';
import { EventContent } from '../src/Timeline';
import { emptyProjection, fromSnapshot, mergeHistory, offeredDecisions, parseBackends, parseEvent, parseHistory,
  parseSession, parseSnapshot, parseTurn, parseProfiles, parseResolvedProfile, projectEvent, safeText, stateExplanation } from '../src/model';
import type { Approval, Session, WorkEvent } from '../src/model';

const sid = '11111111-1111-4111-8111-111111111111';
const tid = '22222222-2222-4222-8222-222222222222';
const aid = '33333333-3333-4333-8333-333333333333';
const token = 'fixture-owner-token-0123456789abcdef';
const session: Session = { id: sid, backend: 'codex', runtime_profile_id: 'codex/local', workspace: 'C:\\projects\\Lain',
  state: 'idle', runtime: 'detached', created_at_ms: 1, updated_at_ms: 2 };
const turn = { id: tid, work_session_id: sid, state: 'completed', cancellation: 'not_requested', created_at_ms: 1, updated_at_ms: 2 };
const approval: Approval = { id: aid, work_session_id: sid, turn_id: tid, kind: 'permissions', description: 'Network operation',
  decisions: ['approve_once', 'deny'], status: 'pending', decision: null, created_at_ms: 1, updated_at_ms: 2 };
const snapshot = { history_cursor: 10, sessions: [session], turns: [turn], approvals: [approval] };
function event(order: number, values: Partial<WorkEvent> = {}): WorkEvent {
  return { id: `00000000-0000-4000-8000-${String(order).padStart(12, '0')}`, work_session_id: sid,
    turn_id: tid, timestamp_ms: order, history_order: order, kind: 'assistant_message_completed', text: `message-${order}`, truncated: false, ...values };
}
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });

describe('public protocol and disposable state', () => {
  it('maps public objects and rejects malformed responses without provider diagnostics', () => {
    expect(parseSession(session)).toEqual(session); expect(parseSnapshot(snapshot)).toEqual(snapshot);
    expect(parseBackends([{ backend: 'codex', available: false, version: null, capabilities: [] }])[0].available).toBe(false);
    expect(parseHistory({ events: [event(1), event(3)], next_cursor: 3, has_more: false }).events).toHaveLength(2);
    for (const value of [{ ...snapshot, history_cursor: 2 ** 54 }, { ...snapshot, turns: null }]) expect(() => parseSnapshot(value)).toThrow('Invalid Kernel response');
    expect(() => parseEvent({ ...event(1), timestamp_ms: 8_640_000_000_000_001 })).toThrow();
    expect(() => parseHistory({ events: [event(3), event(1)], next_cursor: 3, has_more: false })).toThrow();
  });
  it('distinguishes healthy detached, backend disconnected, and uncertain logical work', () => {
    const detached = renderToStaticMarkup(<StateBadges session={session} />);
    expect(detached).toContain('Idle'); expect(detached).toContain('Runtime detached'); expect(detached).not.toContain('Disconnected');
    expect(stateExplanation(session)).toContain('healthy idle');
    expect(stateExplanation({ ...session, state: 'unknown' })).toContain('reliable knowledge');
    expect(stateExplanation({ ...session, state: 'disconnected' })).toContain('not confirmed failed');
    expect(stateExplanation({ ...session, runtime: 'lost' })).toContain('logical session remains');
  });
  it('replaces current state on refresh/reconnect, ignores old live records, keeps history out of projections', () => {
    const initial = fromSnapshot(parseSnapshot(snapshot), 1);
    const running = projectEvent(initial, event(12, { kind: 'session_state_changed', to: 'running' }));
    expect(running.sessions[sid].state).toBe('running');
    expect(projectEvent(running, event(11, { kind: 'session_state_changed', to: 'idle' }))).toBe(running);
    expect(projectEvent(running, event(12, { kind: 'session_state_changed', to: 'idle' }))).toBe(running);
    const refreshed = fromSnapshot(parseSnapshot({ ...snapshot, history_cursor: 20 }), 2);
    expect(refreshed.sessions[sid].state).toBe('idle'); expect(refreshed.sessions[sid].runtime).toBe('detached');
    expect(fromSnapshot(snapshot, 1)).toEqual(fromSnapshot(snapshot, 1));
    expect(projectEvent(emptyProjection(), event(1, { kind: 'session_created', session })).sessions[sid]).toEqual(session);
    const updated = projectEvent(initial, event(11, { kind: 'approval_decision_submitted', approval_id: aid }));
    expect(updated.approvals[aid].status).toBe('responding');
    mergeHistory([], [event(2, { kind: 'session_state_changed', to: 'failed' })]);
    expect(refreshed.sessions[sid].state).toBe('idle');
  });
  it('deduplicates history/live overlap by stable ID and orders by server order, not timestamp', () => {
    const merged = mergeHistory([event(10), event(12)], [event(11, { timestamp_ms: 999 }), event(12)]);
    expect(merged.map(e => e.history_order)).toEqual([10, 11, 12]);
    const large = Array.from({ length: 1100 }, (_, i) => event(i + 1));
    expect(mergeHistory([], large)).toHaveLength(1000);
    expect(mergeHistory([], large)[0].history_order).toBe(101);
    expect(mergeHistory([], large, true)[0].history_order).toBe(1);
    expect(mergeHistory([], [event(0, { kind: 'output_progress', history_order: null })])).toHaveLength(0);
  });
  it('offers only pending decisions advertised by the Kernel', () => {
    const markup = renderToStaticMarkup(<ApprovalCard approval={approval} disabled={false} decide={() => {}} />);
    expect(markup).toContain('Approve once'); expect(markup).toContain('Deny'); expect(markup).not.toContain('>Cancel<');
    expect(offeredDecisions({ ...approval, status: 'responding' })).toEqual([]);
    expect(offeredDecisions({ ...approval, decisions: ['deny', 'deny'] })).toEqual(['deny']);
  });
  it('renders untrusted worker text as text; escapes terminal/bidi controls and bounds giant text', () => {
    const malicious = '<script>globalThis.injected=true</script><img src=x onerror=alert(1)>\x1b[31m\u202e';
    const markup = renderToStaticMarkup(<EventContent event={event(1, { text: malicious })} />);
    expect(markup).toContain('&lt;script&gt;'); expect(markup).not.toContain('<script>'); expect(markup).not.toContain('<img');
    expect(markup).toContain('\\u001b'); expect(markup).toContain('\\u202e');
    expect(safeText('a'.repeat(100000))).toHaveLength(65536 + '\n[Display truncated]'.length);
    const command = renderToStaticMarkup(<EventContent event={event(2, { kind: 'command_completed', command: 'cargo test', output: malicious, exit_code: 101 })} />);
    expect(command).toContain('exit 101'); expect(command).toContain('<details>'); expect(command).not.toContain('<script>');
  });
  it('shows normalized retry and turn failure explanations without declaring observed errors terminal', () => {
    const retry = parseEvent(event(11, { kind: 'error_observed', failure: { kind: 'provider_unavailable', message: 'worker error; backend will retry', retryable: true } }));
    expect(retry.failure?.retryable).toBe(true);
    expect(renderToStaticMarkup(<EventContent event={retry} />)).toContain('backend will retry');
    const running = fromSnapshot({ ...snapshot, sessions: [{ ...session, state: 'running' }], turns: [{ ...turn, state: 'running' }] }, 1);
    const observed = projectEvent(running, retry);
    expect(observed.sessions[sid].state).toBe('running'); expect(observed.turns[tid].state).toBe('running');
    const failure = { kind: 'execution', message: '<script>bad()</script>\x1b' };
    const failed = parseTurn({ ...turn, state: 'failed', failure });
    expect(parseSnapshot({ ...snapshot, turns: [failed] }).turns[0].failure).toEqual(failure);
    for (const e of [parseEvent(event(12, { kind: 'turn_completed', turn: failed })), parseEvent(event(13, { kind: 'backend_disconnected', failure }))]) {
      const markup = renderToStaticMarkup(<EventContent event={e} />);
      expect(markup).toContain('&lt;script&gt;'); expect(markup).toContain('\\u001b'); expect(markup).not.toContain('<script>');
    }
    expect(parseTurn({ ...turn, failure: null })).toEqual(turn);
    expect(() => parseEvent({ ...retry, failure: { kind: 'execution', message: 42 } })).toThrow('Invalid Kernel response');
    expect(() => parseEvent({ ...retry, failure: { kind: 'execution', message: 'safe', retryable: 'true' } })).toThrow('Invalid Kernel response');
  });
  it('parses registry and frozen session views separately and sends only a profile handle', async () => {
    const definition = { id: 'codex/gateway-a', revision: 2, backend: 'codex', driver: 'app-server', enabled: true, model: 'model-new', provider: 'gateway-a', base_url: 'https://example.com/v1', reasoning_effort: 'high', sandbox_default: 'read_only', approval_policy: 'on_request', secret_ref: 'secret://env/gateway-a', health: 'valid', explanation: 'Local prerequisites valid', secret_status: 'available', is_default: false, supported_overrides: ['model', 'sandbox'] };
    expect(parseProfiles([definition])[0]).toEqual(definition);
    const frozen = { profile_id: definition.id, revision: 1, backend: 'codex', driver: 'app-server', model: 'model-old', provider: definition.provider, base_url: definition.base_url, reasoning_effort: 'high', sandbox: 'read_only', approval_policy: 'on_request', secret_ref: definition.secret_ref, secret_status: 'available' };
    expect(parseResolvedProfile(frozen).model).toBe('model-old');
    expect(() => parseProfiles([{ ...definition, revision: -1 }])).toThrow('Invalid Kernel response');
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(session), { headers: { 'content-type': 'application/json' } })); vi.stubGlobal('fetch', fetch);
    await new Client(token).create('/daemon/project', '', '', definition.id);
    expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({ workspace: '/daemon/project', profile_id: definition.id, options: { model: null, sandbox: null } });
  });
});

describe('authenticated transport and recovery', () => {
  it('decodes fragmented Unicode, multiline SSE, heartbeat comments and bounded frames', () => {
    const decoder = new SseDecoder(); const frames: { name: string; data: string }[] = [];
    for (const byte of new TextEncoder().encode('event: event\r\ndata: 你好\r\ndata: again\r\n\r\n: heartbeat\n\nevent: resync_required\ndata: retry\n\n')) frames.push(...decoder.push(new Uint8Array([byte])));
    expect(frames).toEqual([{ name: 'event', data: '你好\nagain' }, { name: 'resync_required', data: 'retry' }]);
    expect(() => new SseDecoder().push(new Uint8Array(32 * 1024 * 1024 + 1).fill(97))).toThrow();
  });
  it('sends detached follow-up through ordinary API and keeps credential out of URLs, objects and error messages', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(turn), { headers: { 'content-type': 'application/json' } }));
    vi.stubGlobal('fetch', fetch); const client = new Client(token);
    await client.prompt(sid, 'continue');
    const [path, options] = fetch.mock.calls[0]; expect(path).toBe(`/v1/work-sessions/${sid}/turns`);
    expect(options.headers.Authorization).toBe(`Bearer ${token}`); expect(options.body).toBe('{"text":"continue"}');
    expect(options.credentials).toBe('omit'); expect(options.redirect).toBe('error'); expect(JSON.stringify(client)).not.toContain(token);
    fetch.mockResolvedValue(new Response('provider-secret-do-not-display', { status: 502 }));
    await expect(client.prompt(sid, 'continue')).rejects.toThrow('Worker operation failed');
    expect(new ApiError(502).message).not.toContain('provider-secret'); expect(() => new Client('secret')).toThrow('Credentials invalid');
  });
  it('takes fresh snapshots on resync and stream closure; abort closes observation without mutation', async () => {
    let calls = 0;
    const fetch = vi.fn().mockImplementation(async (_path, options) => {
      expect(options.headers.Authorization).toBe(`Bearer ${token}`);
      const n = ++calls;
      const wire = `event: snapshot\ndata: ${JSON.stringify({ ...snapshot, history_cursor: n })}\n\n${n === 1 ? 'event: resync_required\ndata: retry\n\n' : ''}`;
      return new Response(new ReadableStream({ start(controller) { for (const byte of new TextEncoder().encode(wire)) controller.enqueue(new Uint8Array([byte])); controller.close(); } }), { headers: { 'content-type': 'text/event-stream' } });
    });
    vi.stubGlobal('fetch', fetch); const controller = new AbortController(); const cursors: number[] = []; let offline = 0;
    await observe(new Client(token), controller.signal, value => {
      if (value.kind === 'snapshot') { cursors.push(value.snapshot.history_cursor); if (cursors.length === 3) controller.abort(); }
    }, () => { offline++; }, 0);
    expect(cursors).toEqual([1, 2, 3]); expect(offline).toBe(2);
    expect(fetch.mock.calls.every(([path, options]) => path === '/v1/events' && options.method === 'GET')).toBe(true);
  });
  it('rejects a stream without authoritative snapshot and stops retrying invalid credentials', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(`event: event\ndata: ${JSON.stringify(event(1))}\n\n`, { headers: { 'content-type': 'text/event-stream' } })));
    await expect(new Client(token).stream(new AbortController().signal, () => {})).rejects.toThrow('Event before snapshot');
    const fetch = vi.fn().mockResolvedValue(new Response('private', { status: 401 })); vi.stubGlobal('fetch', fetch);
    const offline = vi.fn(); await observe(new Client(token), new AbortController().signal, () => {}, offline, 0);
    expect(fetch).toHaveBeenCalledTimes(1); expect(offline).toHaveBeenCalledTimes(1);
  });
});
