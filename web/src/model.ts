// Browser-owned wire views: only normalized public Lain objects.
export interface Session {
  id: string; backend: string; runtime_profile_id: string; workspace: string;
  state: string; runtime: string; created_at_ms: number; updated_at_ms: number;
}
export interface Turn {
  id: string; work_session_id: string; state: string; cancellation: string;
  created_at_ms: number; updated_at_ms: number;
}
export type Decision = 'approve_once' | 'deny' | 'cancel';
export interface Approval {
  id: string; work_session_id: string; turn_id: string | null; kind: string;
  description: string; decisions: Decision[]; status: string; decision: Decision | null;
  created_at_ms: number; updated_at_ms: number;
}
export interface WorkEvent {
  id: string; work_session_id: string; turn_id: string | null; timestamp_ms: number;
  history_order: number | null; truncated: boolean; kind: string;
  session?: Session; turn?: Turn; approval?: Approval; approval_id?: string;
  to?: string; state?: string; text?: string; command?: string; output?: string;
  exit_code?: number | null; paths?: string[]; name?: string; completed?: boolean;
  category?: string; characters?: number; failure?: { kind: string };
}
export interface Snapshot { history_cursor: number; sessions: Session[]; turns: Turn[]; approvals: Approval[] }
export interface HistoryPage { events: WorkEvent[]; next_cursor: number; has_more: boolean }
export interface Backend { backend: string; available: boolean; version: string | null; capabilities: string[] }

const invalid = () => new Error('Invalid Kernel response. Reconnect to obtain a fresh snapshot.');
export function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw invalid();
  return value as Record<string, unknown>;
}
function text(value: unknown): string { if (typeof value !== 'string') throw invalid(); return value; }
function count(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) throw invalid();
  return value;
}
function flag(value: unknown): boolean { if (typeof value !== 'boolean') throw invalid(); return value; }
function timestamp(value: unknown): number {
  const result = count(value);
  if (result > 8_640_000_000_000_000) throw invalid();
  return result;
}
function id(value: unknown): string {
  const result = text(value);
  if (!/^[a-f\d]{8}-(?:[a-f\d]{4}-){3}[a-f\d]{12}$/i.test(result)) throw invalid();
  return result;
}
function list<T>(value: unknown, parse: (item: unknown) => T): T[] {
  if (!Array.isArray(value)) throw invalid();
  return value.map(parse);
}
function decision(value: unknown): Decision {
  if (value !== 'approve_once' && value !== 'deny' && value !== 'cancel') throw invalid();
  return value;
}
export function parseSession(value: unknown): Session {
  const v = object(value);
  return { id: id(v.id), backend: text(v.backend), runtime_profile_id: text(v.runtime_profile_id),
    workspace: text(v.workspace), state: text(v.state), runtime: text(v.runtime),
    created_at_ms: timestamp(v.created_at_ms), updated_at_ms: timestamp(v.updated_at_ms) };
}
export function parseTurn(value: unknown): Turn {
  const v = object(value);
  return { id: id(v.id), work_session_id: id(v.work_session_id), state: text(v.state),
    cancellation: text(v.cancellation), created_at_ms: timestamp(v.created_at_ms), updated_at_ms: timestamp(v.updated_at_ms) };
}
export function parseApproval(value: unknown): Approval {
  const v = object(value);
  return { id: id(v.id), work_session_id: id(v.work_session_id), turn_id: v.turn_id == null ? null : id(v.turn_id),
    kind: text(v.kind), description: text(v.description), decisions: list(v.decisions, decision),
    status: text(v.status), decision: v.decision == null ? null : decision(v.decision),
    created_at_ms: timestamp(v.created_at_ms), updated_at_ms: timestamp(v.updated_at_ms) };
}
export function parseEvent(value: unknown): WorkEvent {
  const v = object(value);
  const event: WorkEvent = { id: id(v.id), work_session_id: id(v.work_session_id),
    turn_id: v.turn_id == null ? null : id(v.turn_id), timestamp_ms: timestamp(v.timestamp_ms),
    history_order: v.history_order == null ? null : count(v.history_order),
    truncated: v.truncated === undefined ? false : flag(v.truncated), kind: text(v.kind) };
  if (v.session !== undefined) event.session = parseSession(v.session);
  if (v.turn !== undefined) event.turn = parseTurn(v.turn);
  if (v.approval !== undefined) event.approval = parseApproval(v.approval);
  if (v.approval_id !== undefined) event.approval_id = id(v.approval_id);
  for (const key of ['to', 'state', 'text', 'command', 'output', 'name', 'category'] as const) {
    if (v[key] !== undefined) event[key] = text(v[key]);
  }
  if (v.paths !== undefined) event.paths = list(v.paths, text);
  if (v.completed !== undefined) event.completed = flag(v.completed);
  if (v.characters !== undefined) event.characters = count(v.characters);
  if (v.exit_code !== undefined) {
    if (v.exit_code !== null && (typeof v.exit_code !== 'number' || !Number.isSafeInteger(v.exit_code))) throw invalid();
    event.exit_code = v.exit_code as number | null;
  }
  if (v.failure !== undefined) event.failure = { kind: text(object(v.failure).kind) };
  return event;
}
export function parseSnapshot(value: unknown): Snapshot {
  const v = object(value);
  return { history_cursor: count(v.history_cursor), sessions: list(v.sessions, parseSession),
    turns: list(v.turns, parseTurn), approvals: list(v.approvals, parseApproval) };
}
export function parseHistory(value: unknown): HistoryPage {
  const v = object(value);
  const events = list(v.events, parseEvent);
  if (events.some((e, i) => e.history_order == null || (i > 0 && e.history_order <= events[i - 1].history_order!))) throw invalid();
  return { events, next_cursor: count(v.next_cursor), has_more: flag(v.has_more) };
}
export function parseBackends(value: unknown): Backend[] {
  return list(value, item => { const v = object(item); return { backend: text(v.backend),
    available: flag(v.available), version: v.version == null ? null : text(v.version), capabilities: list(v.capabilities, text) }; });
}

export const label = (value: string) => value.replaceAll('_', ' ').replace(/^./, s => s.toUpperCase());
export const projectName = (path: string) => path.replace(/[\\/]+$/, '').split(/[\\/]/).at(-1) || path;
export const canSend = (session: Session) => session.state === 'ready' || session.state === 'idle';
export const canCancel = (turn?: Turn) => !!turn && ['starting', 'running'].includes(turn.state)
  && ['not_requested', 'rejected'].includes(turn.cancellation);
export const offeredDecisions = (approval: Approval): Decision[] => approval.status === 'pending' ? [...new Set(approval.decisions)] : [];
export const decisionLabel: Record<Decision, string> = { approve_once: 'Approve once', deny: 'Deny', cancel: 'Cancel' };
export function stateExplanation(session: Session): string | null {
  if (session.state === 'unknown') return "Lain lost reliable knowledge of this worker’s state. Resume / reconcile to ask the backend for current truth.";
  if (session.state === 'disconnected') return 'The backend connection was lost. Work is not confirmed failed. Resume / reconcile before continuing.';
  if (session.runtime === 'lost') return 'The runtime connection was lost. The logical session remains available; follow-up or reconciliation uses the existing session.';
  if (session.runtime === 'detached') return 'Runtime detached — healthy idle work. A follow-up reconnects when needed.';
  return null;
}
// React escapes HTML. Escape non-printing terminal/bidi controls as visible text too.
export function safeText(value: string): string {
  const bounded = value.slice(0, 65536).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/g,
    char => `\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`);
  return bounded + (value.length > 65536 ? '\n[Display truncated]' : '');
}

export function mergeHistory(current: WorkEvent[], incoming: WorkEvent[], older = false): WorkEvent[] {
  const events = new Map(current.map(e => [e.id, e]));
  for (const event of incoming) if (event.history_order !== null) events.set(event.id, event);
  const ordered = [...events.values()].sort((a, b) => a.history_order! - b.history_order!);
  // ponytail: a 1000-record view window; server pagination remains the full history.
  return older ? ordered.slice(0, 1000) : ordered.slice(-1000);
}
export interface Projection {
  sessions: Record<string, Session>; turns: Record<string, Turn>; approvals: Record<string, Approval>;
  orders: Record<string, number>; watermark: number; revision: number;
}
export const emptyProjection = (): Projection => ({ sessions: {}, turns: {}, approvals: {}, orders: {}, watermark: 0, revision: 0 });
export function fromSnapshot(snapshot: Snapshot, revision: number): Projection {
  return { sessions: Object.fromEntries(snapshot.sessions.map(s => [s.id, s])),
    turns: Object.fromEntries(snapshot.turns.map(t => [t.id, t])),
    approvals: Object.fromEntries(snapshot.approvals.map(a => [a.id, a])),
    orders: Object.fromEntries(snapshot.sessions.map(s => [s.id, snapshot.history_cursor])),
    watermark: snapshot.history_cursor, revision };
}
export function projectEvent(current: Projection, event: WorkEvent): Projection {
  const order = event.history_order;
  if (order === null || order <= (current.orders[event.work_session_id] ?? 0)) return current;
  const next = { ...current, sessions: { ...current.sessions }, turns: { ...current.turns },
    approvals: { ...current.approvals }, orders: { ...current.orders, [event.work_session_id]: order } };
  const session = next.sessions[event.work_session_id];
  if (event.session && event.kind === 'session_created') next.sessions[event.session.id] = event.session;
  if (session && event.to) {
    if (event.kind === 'session_state_changed') next.sessions[session.id] = { ...session, state: event.to, updated_at_ms: event.timestamp_ms };
    if (event.kind === 'runtime_attachment_changed') next.sessions[session.id] = { ...session, runtime: event.to, updated_at_ms: event.timestamp_ms };
  }
  if (event.turn) next.turns[event.turn.id] = event.turn;
  if (event.kind === 'cancellation_changed' && event.turn_id && event.state && next.turns[event.turn_id]) {
    next.turns[event.turn_id] = { ...next.turns[event.turn_id], cancellation: event.state };
  }
  if (event.approval) next.approvals[event.approval.id] = event.approval;
  if (event.kind === 'approval_decision_submitted' && event.approval_id && next.approvals[event.approval_id]) {
    next.approvals[event.approval_id] = { ...next.approvals[event.approval_id], status: 'responding' };
  }
  return next;
}
