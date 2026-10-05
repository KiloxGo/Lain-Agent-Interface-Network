import { useCallback, useEffect, useRef, useState } from 'react';
import { ApiError, Client, errorMessage, observe } from './api';
import { emptyProjection, fromSnapshot, mergeHistory, projectEvent } from './model';
import type { Backend, RuntimeProfile, Projection, WorkEvent } from './model';

interface TimelineState {
  session: string | null; events: WorkEvent[]; coveredThrough: number | null;
  hasOlder: boolean; loading: boolean; browsingOlder: boolean;
}
const emptyTimeline = (session: string | null): TimelineState => ({ session, events: [],
  coveredThrough: null, hasOlder: false, loading: false, browsingOlder: false });

export function useControlPlane(client: Client, selected: string | null) {
  const [projection, setProjection] = useState<Projection>(emptyProjection);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<{ ready: string; backends: Backend[]; profiles: RuntimeProfile[] }>({ ready: 'checking', backends: [], profiles: [] });
  const [timeline, setTimeline] = useState<TimelineState>(() => emptyTimeline(selected));
  const [progress, setProgress] = useState<{ session: string; category: string } | null>(null);
  const [reload, setReload] = useState(0);
  const timelineRef = useRef(timeline);
  const selectedRef = useRef(selected); selectedRef.current = selected;
  const generation = useRef(0);
  const observedOrder = useRef(0);
  const olderController = useRef<AbortController | null>(null);
  const changeTimeline = useCallback((fn: (value: TimelineState) => TimelineState) => {
    const value = fn(timelineRef.current); timelineRef.current = value; setTimeline(value);
  }, []);

  useEffect(() => {
    const controller = new AbortController(); let revision = 0;
    const restart = () => setReload(value => value + 1);
    const visible = () => { if (document.visibilityState === 'visible') restart(); };
    window.addEventListener('online', restart); document.addEventListener('visibilitychange', visible);
    setConnected(false);
    void observe(client, controller.signal, update => {
      if (update.kind === 'snapshot') {
        revision += 1;
        observedOrder.current = update.snapshot.history_cursor;
        setProjection(fromSnapshot(update.snapshot, revision));
        setConnected(true); setError(null); setProgress(null);
        const currentRevision = revision;
        void client.status(controller.signal).then(value => {
          if (!controller.signal.aborted && currentRevision === revision) setStatus(value);
        }).catch(error => { if (!controller.signal.aborted) setError(errorMessage(error)); });
      } else {
        const event = update.event;
        observedOrder.current = Math.max(observedOrder.current, event.history_order ?? 0);
        setProjection(current => projectEvent(current, event));
        if (event.kind === 'output_progress') setProgress(current => current?.session === event.work_session_id
          && current.category === (event.category ?? 'worker') ? current : { session: event.work_session_id, category: event.category ?? 'worker' });
        if (event.kind === 'turn_completed' || event.kind === 'backend_disconnected') setProgress(null);
        if (event.work_session_id === selectedRef.current && event.history_order !== null) {
          changeTimeline(previous => {
            const current = previous.session === event.work_session_id ? previous : emptyTimeline(event.work_session_id);
            return { ...current, events: mergeHistory(current.events, [event], current.browsingOlder),
              hasOlder: current.hasOlder || current.events.length >= 1000 };
          });
        }
      }
    }, error => {
      setConnected(false); setProgress(null);
      setError(error instanceof ApiError && error.status === 401 ? error.message : 'Connection lost. Reconnecting; server-side work continues.');
    });
    return () => {
      controller.abort(); window.removeEventListener('online', restart);
      document.removeEventListener('visibilitychange', visible);
    };
  }, [client, changeTimeline, reload]);

  useEffect(() => {
    const controller = new AbortController();
    const request = ++generation.current;
    olderController.current?.abort();
    if (timelineRef.current.session !== selected) changeTimeline(() => emptyTimeline(selected));
    if (!selected || !connected || !projection.revision) return () => controller.abort();
    changeTimeline(value => ({ ...value, loading: true }));
    // Selection may happen long after the stream snapshot. Include all durable
    // activity observed while this session was unselected, then freeze that scan.
    const until = timelineRef.current.coveredThrough === null ? observedOrder.current : projection.watermark;
    const current = () => !controller.signal.aborted && generation.current === request && selectedRef.current === selected;
    void (async () => {
      const covered = timelineRef.current.coveredThrough;
      if (covered === null) {
        const page = await client.history(selected, { before: until + 1, until }, controller.signal);
        if (current()) changeTimeline(value => ({ ...value, events: mergeHistory(value.events, page.events),
          coveredThrough: until, hasOlder: page.has_more }));
      } else {
        // Use the last *fully fetched snapshot* cursor, never the largest live event.
        // A live event can arrive after a missed durable event during disconnect.
        let after = covered;
        for (;;) {
          const page = await client.history(selected, { after, until }, controller.signal);
          if (!current()) return;
          changeTimeline(value => ({ ...value, events: mergeHistory(value.events, page.events, value.browsingOlder),
            hasOlder: value.hasOlder || value.events.length + page.events.length > 1000 }));
          if (!page.has_more) break;
          if (page.next_cursor <= after) throw new Error('History cursor did not advance');
          after = page.next_cursor;
        }
        if (current()) changeTimeline(value => ({ ...value, coveredThrough: until }));
      }
    })().catch(error => { if (current()) setError(errorMessage(error)); })
      .finally(() => { if (current()) changeTimeline(value => ({ ...value, loading: false })); });
    return () => controller.abort();
  }, [client, selected, connected, projection.revision, projection.watermark, changeTimeline]);

  useEffect(() => () => { olderController.current?.abort(); }, []);

  const loadOlder = useCallback(async () => {
    const value = timelineRef.current;
    if (!selected || !connected || value.loading || !value.hasOlder || !value.events.length) return;
    const request = generation.current; const controller = new AbortController();
    olderController.current?.abort(); olderController.current = controller;
    changeTimeline(value => ({ ...value, loading: true }));
    try {
      const page = await client.history(selected, { before: value.events[0].history_order! }, controller.signal);
      if (!controller.signal.aborted && generation.current === request && selectedRef.current === selected) {
        changeTimeline(value => ({ ...value, events: mergeHistory(value.events, page.events, true),
          hasOlder: page.has_more, browsingOlder: true }));
      }
    } catch (error) { if (!controller.signal.aborted && generation.current === request) setError(errorMessage(error)); }
    finally {
      if (!controller.signal.aborted && generation.current === request) changeTimeline(value => ({ ...value, loading: false }));
    }
  }, [client, selected, connected, changeTimeline]);
  const latest = () => {
    olderController.current?.abort(); generation.current += 1;
    changeTimeline(() => emptyTimeline(selected)); setReload(value => value + 1);
  };
  return { projection, connected, error, setError, status, timeline: timeline.session === selected ? timeline : emptyTimeline(selected),
    progress, loadOlder, latest, reconnect: () => setReload(value => value + 1) };
}
