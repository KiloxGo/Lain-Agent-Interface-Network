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

## 7. Phase 1 workspace and local operation

```text
Cargo.toml / Cargo.lock
crates/lain-kernel/
  Cargo.toml
  src/
    lib.rs          library module boundary
    main.rs         laind binary
    domain.rs       IDs, session state and validation
    persistence.rs  private SQLite repository and migrations
    kernel.rs       session projection, mutation and lifecycle ownership
    events.rs       normalized event payloads
    config.rs       daemon settings and API token
    api.rs          authenticated read-only HTTP
  migrations/001_sessions.sql
  tests/foundation.rs
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

All four endpoints require the same bearer token:

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
it. No create endpoint exists in Phase 1; tests exercise internal Kernel methods
to persist representative sessions without inventing a fake worker.

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
recover sessions. There is no continuous polling or model invocation.

## 8. Phase 1 validation and deferred work

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Tests use temporary databases and ephemeral loopback ports, with no external
service or model dependency. They cover the state transition matrix, identifiers,
configuration validation/redaction, persistence/restart, atomic recovery failures,
single ownership, commit-before-event ordering and write failures, subscriber lag,
API authentication/queries/errors, and real TCP shutdown/reopen. No CI workflow
is added.

The Phase 1 implementation was also smoke-tested as a Windows `laind.exe`
process: two boots, authenticated probes/list/inspect, rejected unauthenticated
access, Ctrl-Break with exit code 0, JSON logs without the token, and committed
Running-to-Unknown recovery. The Unix SIGINT/SIGTERM path still needs native
Linux validation; no Linux Rust toolchain was available in this development run.

Deferred: backend adapters and provider identity, session commands over HTTP,
profile/SecretRef registry, durable event history and live transport, worker
process supervision, memory/world, scheduling, Web/TUI, and remote deployment.
The in-process event channel is a notification mechanism, not an audit log.

Before Phase 2, verify the chosen backend's installed protocol/version and decide
how confirmed session completion differs from turn idleness; how disconnects,
cancellation acknowledgements and pending approvals reconcile; where adapter
identity is persisted; and whether owned worker processes stop or detach during
Kernel shutdown. Do not infer these contracts from this metadata-only skeleton.
