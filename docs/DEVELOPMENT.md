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
    profiles.rs     registry definitions, frozen resolution and namespaced secrets
    api.rs          authenticated HTTP control and SSE
    worker.rs       private durable operational metadata
    backend/        supervisor, Codex adapter, JSONL transport, process ownership
  migrations/001_sessions.sql, 002_worker_state.sql, 003_history.sql, 004_profiles.sql
  tests/foundation.rs, worker_control.rs
  tests/support/app_server.rs   deterministic subprocess fixture
scripts/codex_smoke.py          opt-in real daemon/Codex acceptance check
crates/lain-client/
  src/main.rs                  lain commands; environment-only client config
  src/api.rs                   authenticated HTTP/SSE and client wire views
  src/tui.rs                   terminal guard, disposable views and rendering
  tests/cli.rs                 quota-free mock HTTP command acceptance
scripts/terminal_smoke.py       optional Windows ConPTY + fixture acceptance
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

All API endpoints require the same bearer token. Query endpoints:

| GET endpoint | Response |
|---|---|
| `/health` | 200, `{"status":"ok"}` |
| `/ready` | 200 when ready, otherwise 503; `{"state":"ready"}` or current state |
| `/v1/work-sessions` | 200, array of Lain session snapshots (initially `[]`) |
| `/v1/work-sessions/{id}` | 200 snapshot; 400 malformed UUID; 404 absent session |

Missing/incorrect credentials return 401. Session fields are `id`, `backend`,
`runtime_profile_id`, `workspace`, `state`, `created_at_ms`, `updated_at_ms`.
`runtime` reports detached/starting/attached/stopping/lost separately from `state`.
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
recover sessions. Startup resumes only uncertain provider-backed work; known idle
work stays detached. This does not
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

Deferred: additional backends, memory/world, scheduling, automatic history pruning
and remote deployment. Phase 5 profiles and Web are implemented.
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
| `POST /v1/work-sessions/{id}/resume` | Explicit attach/reconcile for Unknown/Disconnected/Idle/Ready; 200 WorkSession |
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
replay output. Snapshots include sessions/turns/approvals and a history_cursor.
Fetch history through that cursor, merge overlapping EventIds in history_order,
and apply current state from the authoritative snapshot/live events. History
records must not overwrite newer projections. Character progress remains live-only.

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

## 11. Operational history

`GET /v1/work-sessions/{id}/history` requires bearer authentication. Schema 003
stores indexed rows with global `sequence`, unique EventId, session, optional turn,
timestamp, kind and bounded normalized JSON payload. Public `history_order` is the
row sequence; it can have gaps within one session and is not a timestamp. IDs and
orders remain stable across detach/restart. Session/worker state and associated
history share one SQLite commit; observation-only history commits before SSE too.
Recovery records are committed with recovery state. No outbox/exactly-once claim.

| Query | Meaning |
|---|---|
| `after=N` | Exclusive lower order; default 0, return oldest matching records first |
| `before=N` | Exclusive upper order; mutually exclusive with after, select nearest older records |
| `until=N` | Inclusive upper watermark; optional, otherwise current high-water mark |
| `limit=N` | 1–200 records, default 100 |

All returned events are ascending in server order, including `before` pages.
Response is `{events, next_cursor, has_more}`. For forward pagination use
`after=next_cursor`. For older pagination use `before=events[0].history_order`;
`has_more` then means older records remain. `next_cursor` is always the last
returned order, or the requested forward cursor if empty. Malformed/oversized
cursors, conflicting directions and invalid limits return 400. Unknown sessions
return 404. Session isolation is applied in SQL. `until` freezes a scan while
other work continues; an SSE snapshot supplies the normal reconnect watermark.

Every normalized event except OutputProgress is durable. Assistant completed
text is limited to 32 KiB UTF-8; command text to 4 KiB and completed output to
16 KiB; file changes to 16 paths of 1 KiB each; tool name to 1 KiB; approval
description to 16 KiB; safe failure messages to 4 KiB. UTF-8 truncation preserves
character boundaries; event `truncated` marks reduced payloads, and pre-bounded
approval descriptions explicitly say truncated. SQLite additionally rejects a
serialized row over 256 KiB (allowing JSON escaping). Sanitize the complete record
before truncation so a credential beyond the visible limit still redacts it.

Raw JSON-RPC, raw provider errors, reasoning, token/output fragments, file diffs,
credential/env dumps and raw prompts are excluded. Pattern/known-value redaction
cannot recognize arbitrary unknown or encoded secrets. History is retained
indefinitely for now; payloads are bounded but total rows/disk usage can grow.
Future pruning must be explicit; summaries and Lain memory are separate features.
Pre-Phase-3 live output cannot be reconstructed retrospectively by migration.

## 12. Terminal client

Build `cargo build --locked --bin laind --bin lain`. Run the daemon as above,
then set the same `LAIN_API_TOKEN` in the client shell, and optionally `LAIN_URL`.
Workspace paths belong to the server and must exist there; the client does not
resolve them using its own filesystem.

```sh
lain status
lain work start --workspace /absolute/path/on/daemon/host
lain work list
lain work show <session-uuid>
lain work attach <session-uuid>
lain work send <session-uuid> "continue and run the tests again"
lain work cancel <session-uuid>
lain work resume <session-uuid>
lain work history <session-uuid> --after 0 --limit 100
lain approval list <session-uuid>
lain approval approve <session-uuid> <approval-uuid>
lain approval deny <session-uuid> <approval-uuid>
lain approval cancel <session-uuid> <approval-uuid>
lain --json work list
```

`--json` is global and available for every non-interactive command; history JSON
includes pagination metadata. Start optionally accepts `--model` and
`--sandbox read-only|workspace-write`. Approval operations recheck Pending and
offered decisions before submitting; the Kernel still authoritatively validates
races. Normal errors show HTTP status/category without echoing credentials or raw
backend response bodies. `lain` without a subcommand opens the work list TUI.

| TUI keys | Operation |
|---|---|
| Up/Down, Enter (list) | Select and observe a WorkSession |
| Enter (detail) | Send edited follow-up only while Idle/Ready and connected |
| Left/Right/Home/End, Backspace/Delete, Ctrl-U | UTF-8 input editing / clear |
| PgUp/PgDn, Ctrl-End | Scroll timeline / follow latest |
| Ctrl-P | Load an older history page within the bounded window |
| Tab, F2 | Next pending approval / full scrollable description |
| Ctrl-A / Ctrl-D / Ctrl-X | Approve once / deny / cancel, only if offered |
| Ctrl-K / Ctrl-R | Cancel active turn / explicitly resume |
| Esc | Close approval details or return to list |
| q (list), Ctrl-Q, Ctrl-C | Exit client; worker remains Kernel-owned |

Attach observes logical work, never a worker terminal. Idle + Detached remains
detached until a prompt/resume requires it. The TUI uses only HTTP/SSE, starts with
the stream's atomic snapshot, loads the latest 200 historical records and merges
by EventId/history_order. It catches up from its last known order on reconnect,
refreshes the authoritative snapshot, and ignores superseded history requests.
Historical records never roll back current projections. It caps the recent window
at 1000 records; CLI/API pagination accesses older rows. This is a disposable view,
not an authoritative second runtime or a transcript database.

Rendering uses Ratatui 0.30.2 + Crossterm 0.29, compatible with the Rust 1.89
floor, per [upstream installation docs](https://ratatui.rs/installation/) verified
2026-10-04. HTTP/SSE uses Reqwest 0.13.5 with Rustls; parsing/CLI uses Clap 4.
Terminal raw/alternate/paste modes restore on normal exits, error returns and panic
hooks. SIGKILL/process termination cannot run restoration. Worker ESC/C1 control
and bidi sequences render as escaped text. Resizing and Unicode editing are tested
with Ratatui's deterministic backend; real Windows ConPTY acceptance is optional:

```powershell
cargo build --locked --bin laind --bin lain --bin lain-test-app-server
python -m venv target/terminal-check-env
target/terminal-check-env/Scripts/python.exe -m pip install pywinpty
target/terminal-check-env/Scripts/python.exe scripts/terminal_smoke.py
```

The ConPTY check also executes the ignored terminal guard test in a native
terminal, covering error returns and caught panic cleanup. It uses the
deterministic worker, no login/model quota. Ordinary
Rust tests need no Python package. Linux terminal/signal/process-group checks
remain native host acceptance work; the available WSL environment lacked a Rust
toolchain in this run. To opt into two real model turns and validate unused detach,
lazy identity-preserving follow-up, CLI history and idle daemon reboot:

```sh
cargo build --locked --bin laind --bin lain
python scripts/codex_smoke.py --run-real --phase3
```

To verify unused Codex binding persistence without inference:
`cargo test -p lain-kernel probe_unused_thread_persistence -- --ignored`.

## 13. Web client

`web/` contains the React/TypeScript/Vite control plane. It uses the same bearer
HTTP/SSE APIs as `lain`, with no backend logic or persistent browser credential
store. Read [WEB.md](WEB.md) for the actual authentication, state-reconstruction,
history pagination, production serving and remote-access boundaries.

Run a daemon with `LAIN_API_TOKEN`, then inside `web`:

```sh
npm ci
npm run dev
npm run typecheck
npm run test
npm run build
```

Vite binds loopback and proxies the Kernel API; `LAIN_DEV_KERNEL_URL` changes its
server-only proxy target. For built serving, set daemon `LAIN_WEB_DIR` to the
absolute `web/dist` directory and open the daemon's root URL. All API routes still
require authentication; static assets alone are public. The UI asks for a token
which remains page-memory-only and is lost on refresh. No TLS/session-cookie/public
Internet readiness is claimed. Phase 5 adds profile selection and frozen inspection.

Deterministic browser acceptance is opt-in and does not consume model quota:

```sh
cargo build --locked --bin laind --bin lain --bin lain-test-app-server
cd web
npx playwright install chromium
npm run test:e2e
```

This uses a private real Kernel, fake worker, temporary data and random credentials;
no existing user service/configuration is mutated. Playwright screenshots are local
ignored artifacts and trace recording is off. Rust/CLI/TUI checks remain the usual
workspace tests and optional terminal smoke. On Windows an active daemon can lock
the normal binary: use `--target-dir target/phase4` and set `LAIN_TEST_BIN_DIR` to
that absolute debug directory for acceptance. Set the matching `CARGO_TARGET_DIR`
when the terminal smoke builds its native guard test. Do not stop a user daemon
merely to overwrite a build artifact.

## 14. Profile and workspace owner API

All routes use the same required owner bearer credential and no-store policy.
Profile IDs may contain slashes, so inspect uses a query argument.

| Method / path | Body / result |
|---|---|
| GET `/v1/runtime-profiles` | Definitions, revisions, local health/explanation, secret status, supported overrides and global-default marker |
| GET `/v1/runtime-profiles/inspect?id=codex/gateway-a` | Current registry view; 404 if absent |
| POST `/v1/runtime-profiles` | ProfileDefinition; 201 definition/revision; duplicate 409 |
| PUT `/v1/runtime-profiles` | `{definition: <full definition>, expected_revision: N}`; new revision or 409 |
| POST `/v1/runtime-profiles/validate` | `{id: "codex/gateway-a"}`; local prerequisites only, zero inference |
| GET `/v1/work-sessions/{id}/resolved-profile` | Frozen session revision/configuration and current SecretRef availability; no provider-native IDs or values |
| GET `/v1/workspace-profile-policy?workspace=<URL-encoded daemon path>` | Canonical policy or null |
| PUT `/v1/workspace-profile-policy` | `{workspace, default_profile, allowed_profiles}`; canonical saved policy |

Create work with `{workspace, profile_id?, options?: {model?, sandbox?}}`.
Omitted/null sandbox now uses the selected profile default, preserving old explicit
read_only/workspace_write requests. The profile determines backend. Disallowed
workspace selection returns 403, absent profile 404, missing credential/backend
503; schema/value failures return safe 400/422. Framework extractor errors are
replaced with safe error codes instead of echoing user values. Metadata-only
historical sessions without a worker snapshot return 404 for resolved-profile
inspection while their ordinary session/history views remain available.

Terminal commands:

```sh
lain profile list
lain profile show codex/gateway-a
lain profile validate codex/gateway-a
lain work start --workspace /daemon/project --profile codex/gateway-a
lain --json profile show codex/gateway-a
```

CLI profile mutations/secret inputs and an interactive TUI profile editor are not
implemented. Owner registry/policy writes use the HTTP API. List/show/attach and
safe failure rendering remain API-only. Web New Work fetches profiles from Kernel,
shows local health/model/endpoint/revision and offers advertised overrides. Session
detail shows its frozen snapshot, distinct from the mutable registry.

Build fixture binaries, then deterministic end-to-end profile acceptance:

```sh
cargo build --locked --bin laind --bin lain --bin lain-test-app-server
python scripts/profiles_smoke.py
```

The test uses its own process environment, private daemon, temporary database and
fake worker. It validates privileged CRUD/revisions, safe malformed input errors,
missing credentials, workspace restrictions, CLI inspection, profile editing,
idle resume and daemon restart with credential rotation and unchanged bindings.
No model quota or real account is used. Rust profiles tests additionally exercise
two simultaneous endpoints/secrets, redaction before history/SSE, and upgrading a
schema-3 database shaped like Phase 4. A failed migration rolls back schema 004.

Native Codex 0.160.0 configuration verification without inference is opt-in:
`cargo test -p lain-kernel probe_profile_launch_configuration -- --ignored`.
It starts two real App Servers with dummy credentials and checks separate model,
provider/base URL/effort configuration through config/read. It does not prove
real provider authentication or model access. No second real gateway credential
was available for paid two-provider acceptance; that check remains unclaimed.
