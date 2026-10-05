import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { Client, errorMessage } from './api';
import { Timeline } from './Timeline';
import { useControlPlane } from './useControlPlane';
import { canCancel, canSend, decisionLabel, label, offeredDecisions, projectName, safeText, stateExplanation } from './model';
import type { Approval, Backend, Decision, Session, Turn } from './model';

function hashSession(): string | null {
  const value = window.location.hash.slice(1);
  return /^[a-f\d]{8}-(?:[a-f\d]{4}-){3}[a-f\d]{12}$/i.test(value) ? value : null;
}
export function StateBadges({ session }: { session: Session }) {
  return <div className="state-badges"><span className={`badge state-${session.state}`}>{label(session.state)}</span>
    <span className={`runtime runtime-${session.runtime}`}>Runtime {label(session.runtime).toLowerCase()}</span></div>;
}
export function ApprovalCard({ approval, disabled, decide }: { approval: Approval; disabled: boolean; decide: (decision: Decision) => void }) {
  return <article className="approval-card" aria-label={`${label(approval.kind)} approval`}>
    <div className="section-bar"><h3>Approval required</h3><span className="badge state-waiting_for_approval">{label(approval.kind)}</span></div>
    <p className="small muted">Review the requested operation before deciding. This permission applies once.</p>
    <pre className="worker-text approval-description" tabIndex={0}>{safeText(approval.description)}</pre>
    <div className="approval-actions">{offeredDecisions(approval).map(decision => <button key={decision}
      className={decision === 'approve_once' ? 'primary' : ''} disabled={disabled} onClick={() => decide(decision)}>{decisionLabel[decision]}</button>)}</div>
  </article>;
}

function Connection({ connect }: { connect: (client: Client) => void }) {
  const [token, setToken] = useState(''); const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null);
  const submit = async (event: FormEvent) => {
    event.preventDefault(); if (busy) return; setBusy(true); setError(null);
    const value = token; setToken('');
    try { const client = new Client(value); await client.request('/health'); connect(client); }
    catch (error) { setError(errorMessage(error)); }
    finally { setBusy(false); }
  };
  return <main className="connection-page"><div className="connection-decoration" aria-hidden="true">LAIN / 04<br />A WINDOW INTO THE WIRED</div>
    <section className="connection-card"><div className="wordmark">lain<span className="wordmark-square">▣</span></div>
      <p className="eyebrow">WORK CONTROL PLANE</p><h1>Your work stays here.</h1>
      <p className="muted">Connect to the Kernel to observe persistent sessions, follow their activity, and decide what happens next.</p>
      <form onSubmit={submit}><label htmlFor="kernel-token">Kernel API token</label><input id="kernel-token" type="password" autoComplete="off"
        spellCheck={false} value={token} onChange={event => setToken(event.target.value)} required minLength={32} maxLength={256} autoFocus />
        <p className="small muted">Kept in this page’s memory only. Refresh requires reconnecting.</p>
        {error && <p role="alert" className="error">{error}</p>}
        <button className="primary connect-button" disabled={busy}>{busy ? 'Connecting…' : 'Connect to Kernel →'}</button>
      </form><div className="connection-footer"><span className="small muted">Same-origin connection</span><span className="small">HTTP + SSE</span></div>
    </section></main>;
}
function NewWork({ backend, disabled, create }: { backend?: Backend; disabled: boolean;
  create: (workspace: string, model: string, sandbox: string) => Promise<void> }) {
  const [workspace, setWorkspace] = useState(''); const [model, setModel] = useState(''); const [sandbox, setSandbox] = useState('read_only');
  const modelSupported = backend?.capabilities.includes('model_selection');
  return <section className="new-work"><p className="eyebrow">NEW WORKSESSION</p><h1>Make room for work.</h1>
    <p className="muted">The Kernel owns this session. It will remain available after you leave.</p>
    <form onSubmit={event => { event.preventDefault(); void create(workspace, modelSupported ? model : '', sandbox); }}>
      <label htmlFor="workspace">Workspace on daemon host</label><input id="workspace" required maxLength={4096} value={workspace} onChange={e => setWorkspace(e.target.value)} placeholder="/home/you/projects/example" autoFocus />
      <p className="small muted">An existing directory on the computer running laind. Browser files are not used.</p>
      {modelSupported && <><label htmlFor="model">Model <span className="muted">(optional)</span></label><input id="model" maxLength={128} value={model} onChange={e => setModel(e.target.value)} placeholder="Use runtime default" /></>}
      <label htmlFor="sandbox">Sandbox</label><select id="sandbox" value={sandbox} onChange={e => setSandbox(e.target.value)}><option value="read_only">Read only</option><option value="workspace_write">Workspace write</option></select>
      <p className="small muted">Backend: {backend?.backend ?? 'Codex'} · profile: codex/local. Runtime profiles arrive in Phase 5.</p>
      {!backend?.available && <p className="error">Codex backend unavailable. Existing history remains readable.</p>}
      <button className="primary" disabled={disabled || !backend?.available || !backend.capabilities.includes('session_create')}>Create WorkSession →</button>
    </form></section>;
}
function SessionDetail({ session, turn, approvals, plane, client, busy, action }: { session: Session; turn?: Turn; client: Client;
  approvals: Approval[]; plane: ReturnType<typeof useControlPlane>; busy: boolean;
  action: (operation: () => Promise<unknown>) => Promise<boolean> }) {
  const [draft, setDraft] = useState(''); const draftRef = useRef(draft); draftRef.current = draft;
  const latestTurn = turn; const ready = plane.connected && plane.status.ready === 'ready';
  const backend = plane.status.backends.find(b => b.backend === session.backend);
  const enabled = ready && !!backend?.available && !busy;
  const explanation = stateExplanation(session);
  const cancelled = latestTurn?.cancellation;
  const submit = async (event: FormEvent) => {
    event.preventDefault(); if (!enabled || !backend?.capabilities.includes('turns') || !canSend(session) || !draft.trim()) return;
    if (new TextEncoder().encode(draft).length > 32768) { plane.setError('Prompt exceeds the Kernel limit of 32 KiB.'); return; }
    const submitted = draft;
    if (await action(() => client.prompt(session.id, submitted)) && draftRef.current === submitted) setDraft('');
  };
  return <div className="session-detail">
    <header className="detail-header"><div><p className="eyebrow">WORKSESSION / {session.id.slice(0, 8)}</p><h1>{safeText(projectName(session.workspace))}</h1>
      <StateBadges session={session} /></div><div className="detail-actions">
      {['unknown', 'disconnected', 'ready', 'idle'].includes(session.state) && <button disabled={!enabled || !backend?.capabilities.includes('session_resume')}
        onClick={() => void action(() => client.resume(session.id))}>Resume / reconcile</button>}
      {canCancel(turn) && <button className="danger-button" disabled={!enabled || !backend?.capabilities.includes('cancel_turn')} onClick={() => void action(() => client.cancel(session.id))}>Cancel work</button>}
    </div></header>
    <dl className="metadata"><div><dt>Workspace</dt><dd>{safeText(session.workspace)}</dd></div><div><dt>Backend / profile</dt><dd>{safeText(session.backend)} / {safeText(session.runtime_profile_id)}</dd></div>
      <div><dt>WorkSession ID</dt><dd>{session.id}</dd></div><div><dt>Latest turn</dt><dd>{turn ? `${turn.id.slice(0, 8)} · ${label(turn.state)}` : 'No turns yet'}</dd></div></dl>
    {explanation && <p className={`state-explanation ${['unknown', 'disconnected'].includes(session.state) ? 'uncertain' : ''}`}>{explanation}</p>}
    {cancelled && cancelled !== 'not_requested' && <p className="state-explanation">Cancellation {label(cancelled).toLowerCase()}{cancelled !== 'confirmed' && ' — interruption is not confirmed.'}</p>}
    {!!approvals.length && <section className="approvals" aria-label="Pending approvals">{approvals.map(approval => <ApprovalCard key={approval.id} approval={approval} disabled={!enabled}
      decide={decision => void action(() => client.decide(session.id, approval.id, decision))} />)}</section>}
    <Timeline key={session.id} {...plane.timeline} loadOlder={() => void plane.loadOlder()} latest={plane.latest} />
    <form className="composer" onSubmit={submit}><div className="section-bar"><label htmlFor="prompt">Follow-up instruction</label>
      <span className="small muted">{plane.progress?.session === session.id ? `Live ${label(plane.progress.category).toLowerCase()} activity` : canSend(session) ? 'Ready for your next instruction' : `Turn ${label(turn?.state ?? session.state).toLowerCase()}`}</span></div>
      <textarea id="prompt" rows={3} maxLength={32768} value={draft} onChange={e => setDraft(e.target.value)} placeholder="Continue and inspect the next step…" disabled={!ready}
        onKeyDown={event => { if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} />
      <div className="composer-footer"><span className="small muted">{canSend(session) ? 'Ctrl / ⌘ + Enter to send' : 'Follow-up available when the active turn ends. No prompt queue.'}</span>
        <button className="primary" disabled={!enabled || !backend?.capabilities.includes('turns') || !canSend(session) || !draft.trim()}>Send instruction ↑</button></div>
    </form>
  </div>;
}

function ControlPlane({ client, disconnect }: { client: Client; disconnect: () => void }) {
  const [selected, setSelected] = useState<string | null>(hashSession);
  const [newWork, setNewWork] = useState(false); const [busy, setBusy] = useState(false); const submitting = useRef(false);
  const plane = useControlPlane(client, selected);
  const select = (id: string | null) => { setSelected(id); setNewWork(false); window.history.replaceState(null, '', id ? `#${id}` : window.location.pathname); };
  useEffect(() => { const sync = () => { setSelected(hashSession()); setNewWork(false); }; window.addEventListener('hashchange', sync); return () => window.removeEventListener('hashchange', sync); }, []);
  const sessions = Object.values(plane.projection.sessions).sort((a, b) => b.updated_at_ms - a.updated_at_ms || a.id.localeCompare(b.id));
  const session = selected ? plane.projection.sessions[selected] : undefined;
  const approvals = Object.values(plane.projection.approvals);
  const turns = Object.values(plane.projection.turns).filter(t => t.work_session_id === selected).reverse().sort((a, b) => b.created_at_ms - a.created_at_ms || b.updated_at_ms - a.updated_at_ms);
  const action = async (operation: () => Promise<unknown>): Promise<boolean> => {
    if (submitting.current || !plane.connected) return false;
    submitting.current = true; setBusy(true); plane.setError(null);
    try { await operation(); return true; }
    catch (error) { plane.setError(errorMessage(error)); return false; }
    finally { submitting.current = false; setBusy(false); }
  };
  return <div className={`app-shell ${selected || newWork ? 'has-detail' : ''}`}>
    <header className="global-header"><a className="wordmark" href="#" onClick={() => select(null)}>lain<span className="wordmark-square">▣</span></a>
      <span className="header-title">WORK / CONTROL PLANE</span><div className="kernel-status" role="status"><span className={`status-dot ${plane.connected ? 'online' : ''}`} aria-hidden="true" />
        <span>{plane.connected ? 'Connected' : 'Reconnecting'} · Kernel {label(plane.status.ready)}</span>
        <span className="backend-status">Codex {plane.status.backends.find(b => b.backend === 'codex')?.available ? 'available' : 'unavailable'}</span></div>
      <button className="text-button" onClick={disconnect}>Disconnect</button>
    </header>
    <aside className="sidebar"><div className="section-bar"><h2>Work sessions</h2><span className="small muted">{sessions.length.toString().padStart(2, '0')}</span></div>
      <button className="new-work-button" disabled={!plane.connected || busy} onClick={() => { setNewWork(true); setSelected(null); window.history.replaceState(null, '', window.location.pathname); }}>+ New Work</button>
      <nav aria-label="Work sessions">{sessions.map(s => {
        const pending = approvals.filter(a => a.work_session_id === s.id && a.status === 'pending').length;
        return <button className={`session-link ${s.id === selected ? 'selected' : ''}`} key={s.id} aria-current={s.id === selected ? 'page' : undefined} onClick={() => select(s.id)}>
          <div className="session-link-title"><strong>{safeText(projectName(s.workspace))}</strong>{pending > 0 && <span className="approval-count">{pending} approval{pending > 1 ? 's' : ''}</span>}</div>
          <div className="session-link-meta">{s.id.slice(0, 8)} · {safeText(s.backend)}</div><StateBadges session={s} />
          <time className="small muted" dateTime={new Date(s.updated_at_ms).toISOString()}>Updated {new Date(s.updated_at_ms).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</time>
        </button>;
      })}</nav>{!sessions.length && <p className="sidebar-empty muted">{plane.connected ? 'No work sessions yet.' : 'Waiting for a Kernel snapshot…'}</p>}
      <footer className="sidebar-footer"><span className="eyebrow">PERSISTENT BY DESIGN</span><p className="small muted">Close this window.<br />Your work stays with the Kernel.</p></footer>
    </aside>
    <main className="main-view"><button className="mobile-back text-button" onClick={() => select(null)}>← Work sessions</button>
      {plane.error && <div className="error-banner" role="alert"><span>{plane.error}</span><button className="text-button" onClick={plane.reconnect}>Reconnect</button></div>}
      {busy && <p className="action-status" role="status">Submitting to Kernel…</p>}
      {newWork ? <NewWork backend={plane.status.backends.find(b => b.backend === 'codex')} disabled={!plane.connected || busy || plane.status.ready !== 'ready'}
        create={async (workspace, model, sandbox) => { await action(async () => { const s = await client.create(workspace, model, sandbox); select(s.id); }); }} />
        : session ? <SessionDetail key={session.id} session={session} turn={turns[0]} approvals={approvals.filter(a => a.work_session_id === session.id && a.status === 'pending')}
          plane={plane} client={client} busy={busy} action={action} />
          : <section className="welcome"><p className="eyebrow">PRESENT DAY / PRESENT TIME</p><h1>A window into<br />persistent work.</h1><p className="muted">{selected && plane.connected ? 'This session is not in the current Kernel snapshot.' : 'Select a session to follow its activity, or create a place for something new.'}</p>
            <div className="welcome-rule" /><p className="small muted">One Kernel. Every interface.<br />Session state and history remain on the daemon.</p></section>}
    </main>
  </div>;
}
export function App() {
  const [client, setClient] = useState<Client | null>(null);
  return client ? <ControlPlane client={client} disconnect={() => setClient(null)} /> : <Connection connect={setClient} />;
}
