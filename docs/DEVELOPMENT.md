# Development Policy

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. Purpose

This document captures repository-level engineering habits that should remain stable while the new implementation is built.

## 2. Local-first development

The primary development loop is a local coding-agent session plus local tools.

Before finishing a change, run the smallest useful deterministic validation locally.

Typical Rust work should prefer:

- `cargo fmt --check`
- targeted `cargo test`
- `cargo clippy` when the changed code benefits from it

Do not run the entire world when a focused test gives the same confidence.

## 3. GitHub Actions

GitHub Actions are optional infrastructure, not the center of the development workflow.

If CI is added later, keep it cheap and deterministic.

Good candidates:

- formatting checks;
- targeted compile/test jobs;
- one primary Linux toolchain;
- lightweight frontend typecheck/lint when needed.

Avoid by default:

- agent/LLM review on every push;
- model calls in CI;
- token-consuming evaluation workflows;
- broad OS/version matrices;
- repeated full integration environments;
- high-frequency scheduled workflows;
- workflows whose runtime cost is larger than their practical signal.

Expensive checks, benchmarks, or agent evaluations should be manually triggered or run for releases when there is a concrete reason.

The goal is fast feedback, not CI theater.

## 4. Milestone commits

The project has an established milestone style: short phase label plus a quoted phrase.

Examples from project history:

- `first echo: "Cogito, Ergo Sum"`
- `birth eve: "I am that I am"`
- `connection: "I and Thou"`
- `woven: "The Thread of Memory"`

Use this style when a commit marks a genuine milestone: a new layer becoming real, a major architecture boundary landing, or a meaningful product capability becoming usable.

Small implementation commits should remain straightforward and descriptive.

Do not force poetic commit messages onto routine fixes.

## 5. Milestone quality bar

A milestone commit should ideally leave the repository in a coherent state:

- relevant docs match the implementation;
- targeted tests pass;
- no known secret is committed;
- temporary scaffolding is either removed or clearly marked;
- the milestone can be explained in one sentence.

## 6. Legacy code

The `legacy/v0` branch preserves the previous implementation.

Do not copy large legacy modules into v1 merely because they already exist.

Legacy code may be consulted for:

- experiments that produced useful lessons;
- old UI/assets;
- Live2D/audio work;
- prior prompt/persona ideas;
- historical behavior.

Any reused code must fit the current architecture rather than pulling the architecture back toward v0.

## 7. Workspace and local operation

```text
Cargo.toml / Cargo.lock
crates/lain-kernel/
  Cargo.toml
  src/
    lib.rs          library module boundary
    main.rs         laind binary
    domain.rs       IDs, sessions, turns, approvals and validation
    persistence.rs  private SQLite repository and migrations
    kernel.rs       session projection, mutation and lifecycle ownership
    events.rs       normalized event payloads
    config.rs       daemon settings and API token
    api.rs          authenticated HTTP control and SSE
    worker.rs       private durable operational metadata
    backend/        supervisor, Codex adapter, JSONL transport, process ownership
  migrations/001_sessions.sql, 002_worker_state.sql
  tests/foundation.rs, worker_control.rs
  tests/support/app_server.rs   deterministic subprocess fixture
scripts/codex_smoke.py          opt-in real daemon/Codex acceptance check
```

Use stable Rust (the workspace language floor is 1.89; the locked dependencies
and checks were tested with Rust 1.98.1 on Windows). Bundled SQLite requires a
working C compiler: MSVC Build Tools on Windows, or a normal C toolchain on Linux.
Cargo.lock is committed for daemon dependency reproducibility.

Linux/macOS shell:

```sh
export LAIN_API_TOKEN="$(openssl rand -hex 32)"
export LAIN_DATABASE_PATH="$PWD/data/lain.sqlite3"
cargo run --locked --bin laind
```

PowerShell:

```powershell
$env:LAIN_API_TOKEN = [Guid]::NewGuid().ToString('N') + [Guid]::NewGuid().ToString('N')
$env:LAIN_DATABASE_PATH = Join-Path $PWD 'data/lain.sqlite3'
cargo run --locked --bin laind
```

For a service, build `cargo build --locked --release --bin laind` and run the
binary directly. Use a private data directory owned by the service user (0700
on Unix, restricted owner ACLs on Windows). The default database path is
`data/lain.sqlite3`; WAL/SHM and lock sidecars live beside it. SQLite migrations
run transactionally using `PRAGMA user_version`; newer unknown schema versions
are refused without downgrade. Back up with the daemon stopped, or use a SQLite
consistent backup mechanism; do not copy only the main file while WAL is active.

All endpoints require the same bearer token. Query endpoints:

| GET endpoint | Response |
|---|---|
| `/health` | 200, `{"status":"ok"}` |
| `/ready` | 200 when ready, otherwise 503; `{"state":"ready"}` or current state |
| `/v1/work-sessions` | 200, array of Lain session snapshots (initially `[]`) |
| `/v1/work-sessions/{id}` | 200 snapshot; 400 malformed UUID; 404 absent session |

Missing/incorrect credentials return 401. Session fields are `id`, `backend`,
`runtime_profile_id`, `workspace`, `state`, `created_at_ms`, `updated_at_ms`.
State names use snake_case. Session lists are ordered by UUID for deterministic
snapshots, not creation time. Pagination is deferred until session volume warrants
it. Provider-native identities are private metadata, not API fields.

In a second shell with the same token available:

```sh
curl -H "Authorization: Bearer $LAIN_API_TOKEN" http://127.0.0.1:8787/ready
curl -H "Authorization: Bearer $LAIN_API_TOKEN" http://127.0.0.1:8787/v1/work-sessions
```

```powershell
$headers = @{ Authorization = "Bearer $env:LAIN_API_TOKEN" }
Invoke-RestMethod http://127.0.0.1:8787/ready -Headers $headers
Invoke-RestMethod http://127.0.0.1:8787/v1/work-sessions -Headers $headers
```

Stop with Ctrl-C (or SIGTERM on Unix). Run again with the same database path to
recover sessions. Startup attempts to resume known Codex identities; this does not
submit a new prompt. Model work starts only from explicit prompt commands.

## 8. Validation and deferred work

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Tests use temporary databases and ephemeral loopback ports, with no external
service or model dependency. They cover the state transition matrix, identifiers,
configuration validation/redaction, persistence/restart, atomic recovery failures,
single ownership, commit-before-event ordering and write failures, subscriber lag,
API authentication/commands/SSE, and real TCP shutdown/reopen. The fake App Server
also tests two sessions, follow-ups, numeric/string approval IDs, permission grants,
cancel rejection/retry and completion races, malformed frames, process crash,
restart/resume, secret fragments, and foreign-session notifications. Migration
from schema 1 and recovery rollback on corrupt worker data are covered. Ordinary
tests never invoke real Codex or a model. No CI workflow is added.

The Phase 1 implementation was also smoke-tested as a Windows `laind.exe`
process: two boots, authenticated probes/list/inspect, rejected unauthenticated
access, Ctrl-Break with exit code 0, JSON logs without the token, and committed
Running-to-Unknown recovery. The Unix SIGINT/SIGTERM path still needs native
Linux validation; no Linux Rust toolchain was available in this development run.

Deferred: additional backends, profile/SecretRef registry, durable output history,
memory/world, scheduling, Web/TUI, and remote deployment.
The in-process event channel is a notification mechanism, not an audit log.

## 9. Worker control API

Install/login to the supported Codex version listed in [CODEX.md](backends/CODEX.md).
If discovery fails, set `LAIN_CODEX_PATH` to the native executable. Query
`GET /v1/backends` for version, availability and adapter capabilities. Availability
does not promise valid authentication or a usable model quota.

| Method and path | Body / result |
|---|---|
| `POST /v1/work-sessions` | `{"workspace":"/absolute/existing/directory","options":{"sandbox":"read_only"}}`; 201 WorkSession |
| `POST /v1/work-sessions/{id}/turns` | `{"text":"instruction"}`; 202 Turn; also used for follow-ups when Idle |
| `GET /v1/work-sessions/{id}/turns` | All durable Turn records |
| `POST /v1/work-sessions/{id}/cancel` | No body; 202 Turn with cancellation acknowledgement, not completion |
| `GET /v1/work-sessions/{id}/approvals` | All Approval records; filter `status == pending` for decisions |
| `POST /v1/work-sessions/{id}/approvals/{approval}/decision` | `{"decision":"approve_once"}`, `deny`, or `cancel` if offered; 202 Approval |
| `POST /v1/work-sessions/{id}/resume` | Explicit retry for Unknown/Disconnected; 200 WorkSession |
| `GET /v1/events` | Authenticated SSE: initial `snapshot`, then normalized `event` records |

Optional model selection is `options.model`. Unknown JSON fields are rejected;
request bodies are limited to 64 KiB and prompt text to 32 KiB. Invalid inputs
return 400/422, absent IDs 404, state conflicts 409, unavailable workers 503, and
backend operation failures 502 with safe error codes. Commands submitted during
an active turn are rejected rather than silently steering/queuing another prompt.

PowerShell example (same daemon token):

```powershell
$headers = @{ Authorization = "Bearer $env:LAIN_API_TOKEN" }
$body = @{workspace = (Get-Location).Path; options = @{sandbox = 'read_only'}} | ConvertTo-Json
$session = Invoke-RestMethod http://127.0.0.1:8787/v1/work-sessions -Method Post -Headers $headers -ContentType application/json -Body $body
$body = @{text = 'Explain the files in this workspace. Do not modify them.'} | ConvertTo-Json
Invoke-RestMethod "http://127.0.0.1:8787/v1/work-sessions/$($session.id)/turns" -Method Post -Headers $headers -ContentType application/json -Body $body
```

SSE clients must use an HTTP client that can send the Authorization header (for
example browser `fetch` streaming; native EventSource cannot set it). Closing a
stream does not stop worker work. Apply the initial snapshot before live events.
On `resync_required`, disconnect and open a new stream; Last-Event-ID does not
replay output. Durable snapshots include sessions/turns/approvals, not transcripts.
On `backend_reconnected`, refresh that session's turns and approvals. Character
progress is live-only; completed text is sanitized and bounded.

## 10. Optional real Codex smoke

This explicitly consumes model quota and uses the local Codex login/default model.
It creates only temporary workspaces and its own database/token; Codex may retain
its native rollout history in its own home. It never rewrites global config.
Use Windows or Linux, Python 3 and the supported authenticated native Codex binary:

```sh
cargo build --locked --bin laind
python scripts/codex_smoke.py --run-real
```

The script exercises HTTP/SSE, two real provider sessions, initial/follow-up turns,
scoped cancellation, owner approval, client detach/reconnect, owned process crash,
resume, daemon restart and identity preservation. It uses a private executable
copy so Windows builds remain possible while it runs. Approval generation is
model-dependent; failure to request approval fails the smoke rather than silently
skipping that acceptance check. Do not include this command in ordinary CI.
