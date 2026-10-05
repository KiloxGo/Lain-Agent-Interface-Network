import { useLayoutEffect, useRef } from 'react';
import { decisionLabel, label, safeText } from './model';
import type { WorkEvent } from './model';

export function EventContent({ event: e }: { event: WorkEvent }) {
  switch (e.kind) {
    case 'assistant_message_completed': return <><span className="event-label">Assistant</span><div className="worker-text assistant-text">{safeText(e.text ?? '')}</div></>;
    case 'command_started': return <><span className="event-label">Command started</span><code className="worker-text command">$ {safeText(e.command ?? '')}</code></>;
    case 'command_completed': return <><span className="event-label">Command · exit {e.exit_code ?? 'unknown'}</span>
      <code className="worker-text command">$ {safeText(e.command ?? '')}</code>
      {!!e.output && <details><summary>Command output</summary><pre className="worker-text output">{safeText(e.output)}</pre></details>}</>;
    case 'file_changed': return <><span className="event-label">Files changed</span><ul className="file-paths">{e.paths?.map((path, i) => <li key={i}><code>{safeText(path)}</code></li>)}</ul></>;
    case 'turn_started': case 'turn_completed': case 'turn_state_changed': return <><span className="event-label">Turn {label(e.turn?.state ?? 'unknown')}</span>
      <span className="event-note">{e.turn?.id.slice(0, 8)}{e.turn?.cancellation !== 'not_requested' && ` · cancellation ${label(e.turn?.cancellation ?? 'unknown')}`}</span>
      {e.turn?.failure && <div className="worker-text">{safeText(e.turn.failure.message)}</div>}</>;
    case 'cancellation_changed': return <><span className="event-label">Cancellation {label(e.state ?? 'unknown')}</span>
      <span className="event-note">{e.state === 'confirmed' ? 'Worker interruption confirmed' : 'A request or acknowledgement does not confirm interruption'}</span></>;
    case 'approval_requested': return <><span className="event-label">Approval requested · {label(e.approval?.kind ?? 'operation')}</span><div className="worker-text">{safeText(e.approval?.description ?? '')}</div></>;
    case 'approval_resolved': case 'approval_state_changed': return <><span className="event-label">Approval {label(e.approval?.status ?? 'unknown')}</span>
      <span className="event-note">{e.approval?.decision ? decisionLabel[e.approval.decision] : 'No confirmed decision'}</span></>;
    case 'approval_decision_submitted': return <span className="event-label">Approval decision submitted · awaiting resolution</span>;
    case 'runtime_attachment_changed': return <span className="event-note">Runtime {label(e.to ?? 'unknown')}</span>;
    case 'session_state_changed': return <span className="event-note">Session {label(e.to ?? 'unknown')}</span>;
    case 'session_created': return <span className="event-note">WorkSession created</span>;
    case 'backend_disconnected': case 'error_observed': return <><span className="event-label">{e.kind === 'backend_disconnected' ? 'Backend connection lost' : 'Worker error'} · {label(e.failure?.kind ?? 'unknown')}</span>
      {e.failure && <div className="worker-text">{safeText(e.failure.message)}</div>}</>;
    case 'backend_reconnected': return <span className="event-note">Backend reconnected · state reconciled by Kernel</span>;
    case 'tool_activity': return <span className="event-note">Tool {safeText(e.name ?? '')} · {e.completed ? 'completed' : 'started'}</span>;
    default: return <span className="event-note">{safeText(label(e.kind))}</span>;
  }
}
export function Timeline({ events, loading, hasOlder, browsingOlder, loadOlder, latest }: {
  events: WorkEvent[]; loading: boolean; hasOlder: boolean; browsingOlder: boolean;
  loadOlder: () => void; latest: () => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const previous = useRef({ first: 0, height: 0 });
  const follow = useRef(true);
  useLayoutEffect(() => {
    const element = container.current;
    if (!element) return;
    const first = events[0]?.history_order ?? 0;
    if (previous.current.first && first < previous.current.first) element.scrollTop += element.scrollHeight - previous.current.height;
    else if (follow.current) element.scrollTop = element.scrollHeight;
    previous.current = { first, height: element.scrollHeight };
  }, [events]);
  return <section className="timeline-region" aria-label="Operational timeline">
    <div className="section-bar"><h2>Operational timeline</h2><div className="inline-actions">
      <span className="small muted">{events.length} records</span>
      <button className="text-button" onClick={() => { follow.current = true; latest(); }}>Latest activity ↓</button>
    </div></div>
    <div className="timeline" ref={container} tabIndex={0} aria-label="Scrollable timeline" onScroll={event => {
      const element = event.currentTarget;
      follow.current = element.scrollHeight - element.scrollTop - element.clientHeight < 70;
      if (element.scrollTop < 60 && hasOlder && !loading) loadOlder();
      previous.current.height = element.scrollHeight;
    }}>
      {hasOlder && <button className="older-button" disabled={loading} onClick={loadOlder}>{loading ? 'Loading history…' : 'Load older history ↑'}</button>}
      {browsingOlder && <p className="window-note">Viewing older history. Use Latest activity to return to current work.</p>}
      {!events.length && <div className="timeline-empty">{loading ? 'Reconstructing history…' : 'No operational records yet.'}</div>}
      <ol className="events">{events.map(e => <li key={e.id} className={`event event-${e.kind}`} data-event-id={e.id}>
        <time dateTime={new Date(e.timestamp_ms).toISOString()} title={new Date(e.timestamp_ms).toLocaleString()}>{new Date(e.timestamp_ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</time>
        <div className="event-body"><EventContent event={e} />{e.truncated && <span className="truncated">Output bounded by Kernel · truncated</span>}</div>
      </li>)}</ol>
    </div>
  </section>;
}
