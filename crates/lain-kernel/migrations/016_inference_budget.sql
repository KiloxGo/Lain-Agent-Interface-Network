CREATE TABLE inference_budgets (
    id TEXT PRIMARY KEY,
    snapshot TEXT NOT NULL
);
CREATE TABLE inference_decisions (
    revision INTEGER PRIMARY KEY,
    snapshot TEXT NOT NULL
);
CREATE TABLE inference_requests (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    request_number INTEGER NOT NULL,
    budget_id TEXT REFERENCES inference_budgets(id),
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    UNIQUE(actor_turn_id,request_number)
);
CREATE INDEX inference_budget_usage ON inference_requests(budget_id,sequence);
CREATE INDEX inference_pending ON inference_requests(state);
PRAGMA user_version=16;
