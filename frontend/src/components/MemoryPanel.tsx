import { useCallback, useEffect, useState } from "react";

type MemoryItem = {
  id: string;
  memory_type: string;
  scope: string;
  subject: string;
  predicate: string;
  value: string;
  sensitivity: string;
  status: string;
  version: number;
  source_message_id: string;
  evidence_quote: string;
  never_recall: boolean;
  pinned: boolean;
  updated_at: string;
};

type PendingMemory = {
  id: string;
  draft: {
    memory_type: string;
    scope: string;
    subject: string;
    predicate: string;
    value: string;
    evidence_quote: string;
    sensitivity: string;
  };
  decision_reason: string;
  source_message_id: string;
};

type MemoryOverview = { items: MemoryItem[]; pending: PendingMemory[] };

function MemoryPanel() {
  const [overview, setOverview] = useState<MemoryOverview>({ items: [], pending: [] });
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const response = await fetch("/api/memory");
    if (!response.ok) throw new Error(`memory request failed (${response.status})`);
    setOverview((await response.json()) as MemoryOverview);
  }, []);

  useEffect(() => {
    refresh().catch((reason: unknown) =>
      setError(reason instanceof Error ? reason.message : String(reason)),
    );
  }, [refresh]);

  const mutate = async (id: string, path: string, init: RequestInit) => {
    setBusyId(id);
    setError(null);
    try {
      const response = await fetch(path, init);
      if (!response.ok) throw new Error(`memory update failed (${response.status})`);
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusyId(null);
    }
  };

  const updateItem = (item: MemoryItem, update: Record<string, boolean | string>) =>
    mutate(item.id, `/api/memory/${encodeURIComponent(item.id)}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(update),
    });

  return (
    <details>
      <summary>
        Memory ({overview.items.length} active, {overview.pending.length} awaiting confirmation)
      </summary>
      {error ? <p role="status">{error}</p> : null}
      {overview.pending.length > 0 ? <h2>Awaiting confirmation</h2> : null}
      {overview.pending.map((candidate) => (
        <article key={candidate.id}>
          <p>
            <strong>{candidate.draft.value}</strong> · {candidate.draft.memory_type} ·{" "}
            {candidate.draft.sensitivity}
          </p>
          <p>Evidence: “{candidate.draft.evidence_quote}”</p>
          <p>Reason: {candidate.decision_reason}</p>
          <button
            type="button"
            disabled={busyId === candidate.id}
            onClick={() =>
              void mutate(
                candidate.id,
                `/api/memory/pending/${encodeURIComponent(candidate.id)}/confirm`,
                { method: "POST" },
              )
            }
          >
            Confirm
          </button>
          <button
            type="button"
            disabled={busyId === candidate.id}
            onClick={() =>
              void mutate(
                candidate.id,
                `/api/memory/pending/${encodeURIComponent(candidate.id)}`,
                { method: "DELETE" },
              )
            }
          >
            Reject
          </button>
        </article>
      ))}
      {overview.items.length > 0 ? <h2>Committed memory</h2> : <p>No committed memory.</p>}
      {overview.items.map((item) => (
        <article key={item.id}>
          <p>
            <strong>{item.value}</strong> · {item.memory_type} · {item.scope} · v{item.version}
          </p>
          <p>Evidence: “{item.evidence_quote}”</p>
          <small>Source message: {item.source_message_id}</small>
          <div>
            <button
              type="button"
              disabled={busyId === item.id}
              onClick={() => void updateItem(item, { pinned: !item.pinned })}
            >
              {item.pinned ? "Unpin" : "Pin"}
            </button>
            <button
              type="button"
              disabled={busyId === item.id}
              onClick={() => void updateItem(item, { never_recall: !item.never_recall })}
            >
              {item.never_recall ? "Allow recall" : "Never recall"}
            </button>
            <button
              type="button"
              disabled={busyId === item.id}
              onClick={() => void updateItem(item, { status: "disputed" })}
            >
              Dispute
            </button>
            <button
              type="button"
              disabled={busyId === item.id}
              onClick={() =>
                void mutate(item.id, `/api/memory/${encodeURIComponent(item.id)}`, {
                  method: "DELETE",
                })
              }
            >
              Retract
            </button>
          </div>
        </article>
      ))}
    </details>
  );
}

export default MemoryPanel;
