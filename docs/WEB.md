# Web control plane

Status: implemented Phase 4. Verified 2026-10-05 on Windows / Chromium.

The Web application is a disposable client of `laind`. It has no worker adapter,
SQLite access, local transcript database, prompt queue or backend privileges.
Closing, refreshing or opening another tab changes only browser observation.
Phase 5 profiles and Lain Actor remain unimplemented.

## Stack and layout

React 19, TypeScript 5 and Vite 8 provide a small client without Redux, router,
GraphQL, WebSocket or component framework. The exact versions are in
`web/package-lock.json`. React/Vite usage follows their
[React](https://react.dev/learn) and [Vite](https://vite.dev/guide/) documentation,
checked 2026-10-05. Node 22.12+ is required; Node 24 was used locally.

```text
web/
  src/
    api.ts                 bounded authenticated fetch / SSE transport
    model.ts               public wire parsing and disposable projections
    useControlPlane.ts     observation, reconnect and history fetching
    App.tsx                connection, shell, list, creation, detail, approvals
    Timeline.tsx           normalized timeline and scroll anchoring
    styles.css             responsive control-plane design
    main.tsx               entry point
  public/mark.svg           code-native identity mark
  tests/control.test.tsx    deterministic transport/state/rendering checks
  tests/e2e/control.spec.ts opt-in browser + real Kernel + fake worker
scripts/web_fixture.py     isolated daemon for browser acceptance
```

The desktop shell has a session sidebar and a detail area. List rows show project,
short Lain ID, backend, logical state, separately labelled runtime attachment,
pending approval count and the Kernel's update timestamp. Detail shows full ID,
daemon-host workspace, backend/profile, latest turn, cancellation evidence,
pending approvals, timeline and follow-up composer. There are no native thread IDs.
Kernel readiness and backend availability are separate from browser connectivity.
Timeline errors and turn failures display the Kernel's normalized safe explanation,
including backend retry notices. An observed error does not imply a failed turn;
terminal state still comes from Kernel turn evidence. Raw provider diagnostics are
not exposed.
On phones, list and detail occupy separate screens with a back button; timeline,
approval descriptions and command output scroll in bounded regions. Keyboard focus,
semantic buttons, labels and text state labels supplement color. No animations or
external font/image services are required; reduced-motion preferences are respected.

## Authentication and origin

Phase 4 uses an explicit password-style token entry screen. `LAIN_API_TOKEN` is
held only inside the live page's Client object and sent as an Authorization bearer
header. The input is cleared on submission. No token is stored in localStorage,
sessionStorage, IndexedDB, cookies, URLs, bundles or logs. Disconnect drops the
client and aborts observation; refresh requires re-entry. The selected public
WorkSession UUID alone can appear in the URL hash for refresh reconstruction.
Workers never receive the daemon credential.

All API routes, including health/readiness/SSE, keep bearer authentication.
Only static assets are public. API failures display status-based safe explanations,
not raw response bodies or provider diagnostics. Invalid credentials stop automatic
retry. The connection screen explains invalid credentials and an unreachable
Kernel; the shell reports not-ready and unavailable backends independently.

This is a deliberate local/interim flow, not a completed remote session-auth design.
The browser must load trusted first-party code; an XSS, malicious extension or
compromised machine can access in-memory credentials. JavaScript cannot promise
secure erasure. No CORS exception or authentication bypass is installed.
See [ADR 002](adr/002-web-origin-and-authentication.md) for the longer-term boundary.

## Local development

Run `laind` with its usual daemon token and loopback listener, then:

```sh
cd web
npm ci
npm run dev
```

Open the loopback URL printed by Vite and enter that daemon token. The Vite server
proxies `/health`, `/ready` and `/v1` to `http://127.0.0.1:8787`. If necessary set
`LAIN_DEV_KERNEL_URL` in the Vite server environment. This variable is server-only;
the browser uses same-origin relative API paths. Vite never reads the API token.
Do not put credentials in `VITE_*` variables. Keep this development server local.
The production security policy below applies to the built assets served by `laind`,
not to Vite's development/HMR server.

```sh
npm run typecheck
npm run test
npm run build
```

TypeScript strict checking is the default static code check; there is no redundant
lint/format plugin stack. Ordinary Vitest tests need neither a daemon nor Codex.

## Production build and serving

```sh
cd web
npm ci
npm run build
cd ..
cargo build --locked --release --bin laind --bin lain
```

Set `LAIN_WEB_DIR` to the absolute `web/dist` directory, then run `laind`. It
validates that directory and its `index.html` at startup and serves static files
on the same loopback listener as the API. Without `LAIN_WEB_DIR`, headless operation
remains available. No nginx or Node server is required for this serving model.
Deploy the whole built directory, not only index.html; do not point this setting
at the repository, data directory or any directory containing private files.
The server uses [Tower HTTP ServeDir](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html)
for normal filesystem serving/path rejection. Root-path hosting is supported;
subpath hosting and cross-origin Kernel selection are not configured in Phase 4.
Selection uses hashes, so arbitrary SPA route fallbacks are unnecessary.

Static and API responses use no-store. Web responses add nosniff, no-referrer and
a CSP restricted to first-party script/style/connect sources, with no inline
script, objects, base changes or frame embedding. The built application uses
external script and CSS assets compatible with that policy.

## Reconstruction, SSE and pagination

The fetch-based SSE transport supports bearer headers; native EventSource does
not expose that header option. See the
[SSE framing reference](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events).
No browser-specific domain API was needed. Model selection is now explicitly
advertised as a capability of the existing Codex session-options API.

1. Connect to `/v1/events` and apply its atomic authoritative snapshot: sessions,
   turns, approvals and `history_cursor`. Replace previous projections completely.
2. For a newly selected session, freeze a watermark at the greater of the snapshot
   cursor and highest durable order already observed on this stream, then fetch the latest
   200 records with `before=watermark+1&until=watermark`. A session may have changed
   while unselected or been created after the initial snapshot. Events arriving
   during selection are retained too. This is a bounded recent window.
3. Merge fetched records and live durable events by EventId; sort by server
   `history_order`, never timestamps. Historical records contribute only timeline
   entries and never mutate current state. Per-session order guards stop delayed
   duplicate live records from rolling newer projections back.
4. After disconnect/resync, reconnect for a fresh snapshot. Catch up with
   `after=<last fully fetched history watermark>&until=<new snapshot cursor>`, following
   forward pages to completion. A largest live event is not a coverage cursor:
   earlier events can have been missed while offline. Failed/aborted catch-up never
   advances the completed coverage watermark.
5. Load older records when the user scrolls upward or uses Load older history,
   with `before=<oldest shown order>`. Preserve the scroll anchor. A 1000-record
   window bounds rendering. While browsing older records, preserve that window;
   Latest activity returns to a fresh recent window. The API/CLI can inspect the
   full retained history.

Abort and generation guards ignore stale history requests after selection changes,
refresh or reconnect. Observation and older-page fetching have separate abort
controllers. Live OutputProgress shows activity only; it is never stored as a
browser transcript. JSON responses and SSE frames are limited to 32 MiB. History
cursors must fit JavaScript safe integers (up to 2^53-1); larger cursors fail
explicitly rather than lose ordering precision. Kernel bounds remain in force.

Connection loss displays reconnecting and disables mutations without changing
session truth. A new snapshot is required after `resync_required`. Fetch retries
after two seconds; a 45-second stream inactivity timeout, visibility return and
browser online events recover sleep/network loss. HTTP commands have a 60-second
bound. Closing the observer sends no cancel/stop/resume command. Two tabs can
observe the same detached work without launching a runtime.

## Commands and domain semantics

New Work sends a daemon-host workspace path, optional supported model, and
read-only/workspace-write sandbox through the normal creation API. It performs
no browser filesystem validation. The current profile label is `codex/local`.
Ready/Idle sessions allow a follow-up through the ordinary prompt endpoint;
detached attachment is the Kernel's responsibility. Active turns disable concurrent
submission; no frontend queue is introduced. Ctrl/Command+Enter submits; accepted
prompts clear only the unchanged submitted draft. Pending HTTP operations disable
duplicate controls while the Kernel remains responsible for cross-client races.

Approval cards show category, full bounded normalized operation/description and
only the actual Pending decisions: Approve once, Deny and applicable Cancel.
Responding, expired, unknown or resolved requests offer no action. Decisions go
through the same scoped Kernel endpoint and are revalidated by the Kernel;
they do not imply execution succeeded. There is no auto-approval.

Unknown means unreliable knowledge, not failure. Disconnected means lost backend
connection, not confirmed failure. Detached means temporary runtime released and
can be healthy. Cancellation requested/acknowledged is distinct from confirmed
interruption. Resume / reconcile explicitly requests the existing session's truth.

Timeline renders completed assistant messages, expandable bounded command output
and exit status, changed paths, turn/cancellation lifecycle, approvals, compact
tool activity and backend/runtime events. No raw JSON or provider errors are shown.
React text nodes escape HTML; unsafe HTML APIs and Markdown/ANSI HTML converters
are absent. Terminal/C1/bidi controls become visible escapes. Text has a further
64-Ki-code-unit display ceiling; wrapped text and bounded scroll containers protect
the layout. Unrecognized event kinds render a text label without executing content.

## Acceptance and limitations

After building the Rust fixture binaries and Web assets:

```sh
cargo build --locked --bin laind --bin lain --bin lain-test-app-server
cd web
npx playwright install chromium
npm run test:e2e
```

The opt-in Playwright suite launches a private real daemon with the deterministic
Rust worker, temporary workspace/database and random token. It covers creation,
live output, command/file history, deny and approve-once choices, cancellation,
browser closure during active work, two observers, refresh, daemon restart,
idle detach, follow-up, pagination, network loss/catch-up, Unknown reconciliation,
HTML/control injection, desktop/laptop/tablet/phone layout and empty browser credential storage.
Screenshots remain ignored local test artifacts. Trace recording is disabled.
No login, provider network or model quota is used; no CI workflow was added.

If an existing Windows daemon locks `target/debug/laind.exe`, use an isolated
Cargo target directory and set `LAIN_TEST_BIN_DIR` to its absolute `debug` directory
for browser/terminal acceptance. The terminal guard test can use the matching
`CARGO_TARGET_DIR`; no running user daemon needs to be stopped.

Remote access remains explicit: the daemon still binds loopback only. An SSH
tunnel can expose that loopback service to the owner's laptop. A private network
or TLS reverse proxy must provide trusted transport and access control before
broader use; do not send the bearer token over ordinary remote plaintext HTTP.
Tailscale/private networking alone is not a public-browser authentication design.
Native phone hosting, public Internet authentication, multi-user access, session
cookies, TLS termination, origin/CSRF policy and token rotation are not implemented
or claimed reviewed. Native Linux worker/signal/terminal acceptance remains pending.

Before Phase 5, settle profile capability/resolved-view APIs, browser session
authentication/rotation if remote use is required, and a TLS/origin deployment
security review. These questions do not justify implementing profiles in Phase 4.
