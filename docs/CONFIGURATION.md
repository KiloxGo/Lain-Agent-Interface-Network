# Configuration and Runtime Profiles

## Target configuration ownership — 2026-10-23

[RESIDENT_DESIGN.md](RESIDENT_DESIGN.md) extends the target to one cloud identity
and paired devices. Cloud identity/persona, scoped grants, channel bindings,
selected source lists and versioned reusable resources are separate from native
device paths, executable installations and device-local credentials.
No configuration keys for these proposed capabilities exist merely because this
document describes them. The settings below remain the current local implementation.

Do not synchronize entire configuration/credential directories or rewrite user
global Worker settings. Future resource sync uses explicit versions; in-flight
work retains its selected revision unless a new instruction changes it.
QQ observer and conversation accounts need distinct action/target capabilities.
Budget values, pairing, key rotation and grant formats remain design decisions.

**Status:** target ownership update plus historical profile design/current configuration
**Last reviewed:** 2026-10-23

## 1. Purpose

Lain should launch worker agents with different providers, API endpoints, models, policies, and credentials without forcing the owner to manually edit each worker's global configuration.

This document defines the profile model.

## 2. Principle

> Configuration belongs to named runtime profiles. Secrets belong to a secret store.

A profile describes how to start or control a worker. It does not contain raw secret values.

## 3. RuntimeProfile

The following are architectural examples for future adapters, not the supported
input format. Sections 25–28 define the implemented Phase 5 API and env-only
SecretRef namespace; TOML loading and the other backends remain deferred.

Conceptual examples:

~~~toml
[profiles."codex/official"]
backend = "codex"
driver = "app-server"
model = "gpt-5.6"
reasoning_effort = "high"
secret_ref = "secret://openai/main"

[profiles."claude/gateway-a"]
backend = "claude-code"
driver = "acp"
model = "claude-sonnet"
base_url = "https://gateway.example.com"
secret_ref = "secret://anthropic/gateway-a"

[profiles."deepseek/official"]
backend = "deepseek-harness"
driver = "acp"
secret_ref = "secret://deepseek/main"
~~~

Exact syntax is not frozen.

## 4. Profile fields

Common fields may include:

- profile ID;
- backend;
- driver;
- model;
- base URL/provider;
- reasoning/effort;
- approval policy;
- sandbox policy;
- environment overrides;
- secret references;
- timeout/resource limits;
- backend-specific options.

Do not force every backend to support every field.

## 5. Configuration precedence

Recommended precedence:

~~~text
built-in defaults
    <
system configuration
    <
named RuntimeProfile
    <
workspace policy overrides
    <
explicit per-session request
~~~

Secret values are resolved separately.

The final resolved configuration should be inspectable with secrets redacted.

## 6. Launch-time injection

Prefer session/runtime-scoped configuration.

~~~text
resolve profile
   |
resolve SecretRefs
   |
build process environment / protocol options
   |
launch worker runtime
~~~

Avoid mutating global user configuration before each launch.

Global mutation creates races when multiple sessions use different profiles at the same time.

## 7. Global config interoperability

Lain may later support:

- importing known CLI configs;
- exporting a profile;
- reading existing environment settings;
- interoperability with external switcher tools.

These should be explicit compatibility features, not the primary session-routing mechanism.

## 8. SecretRef

Profiles reference secrets:

~~~text
secret://openai/main
secret://anthropic/official
secret://anthropic/gateway-a
~~~

A SecretRef may be persisted.

A resolved secret value:

- must not be returned through ordinary profile APIs;
- must not enter Lain memory;
- must be redacted from logs;
- should be injected only when needed;
- should have the smallest practical lifetime.

## 9. Secret backend

Possible initial implementations:

- protected local file;
- systemd credentials;
- OS keyring;
- age-encrypted file.

Future integrations may use a dedicated vault.

For a personal Linux server, a simple local backend is acceptable initially if file permissions are strict, backups are considered, logs redact values, and public APIs never expose plaintext.

Do not store raw secrets in ordinary SQLite profile rows.

## 10. Provider/API switching

The user-facing abstraction is profile selection, not "edit Claude config" or "edit Codex config".

Examples:

~~~text
Use Claude through gateway-a.
Use official Codex.
Use the cheap profile.
~~~

Lain may later map intent/policy to a profile. Early versions should prefer explicit selection for predictability.

## 11. Policy-based routing

Future routing may consider:

- owner preference;
- task type;
- cost;
- provider availability;
- rate limits;
- workspace policy;
- privacy constraints;
- backend health.

~~~text
WorkRequest
  |
  v
RoutingPolicy
  |
  +-- codex/official
  +-- claude/gateway-a
  `-- deepseek/official
~~~

Automatic routing must be explainable, observable, and overrideable.

Never silently route sensitive work through a third-party gateway without explicit policy.

## 12. Workspace policy

A workspace may constrain profiles.

Example:

~~~toml
[workspaces.Klog]
path = "/home/user/codebase/Klog"
allowed_profiles = ["codex/official", "claude/official"]
default_profile = "codex/official"
~~~

Future restrictions may include:

- deny external gateways;
- force local model;
- require network approval;
- read-only sessions;
- one mutating session at a time.

## 13. Backend-native options

Some options have no common equivalent.

Store them under a backend-specific extension namespace.

The adapter owns interpretation.

Core services should treat these fields as opaque validated configuration rather than grow provider-specific branches.

## 14. Dynamic configuration

Some runtimes may support model or option changes during a live session.

Do not assume this universally.

Represent it as a backend capability.

If unsupported, use a new session/profile rather than silently pretending a live change succeeded.

## 15. Validation

Validate profiles before work begins.

Validation levels:

1. schema validation;
2. SecretRef existence;
3. driver/executable availability;
4. optional provider health/auth probe.

A profile can be represented as:

~~~text
Valid
Invalid
Unavailable
Unknown
~~~

Errors should be actionable.

## 16. Redacted resolved view

The owner should be able to inspect a resolved profile without seeing credentials.

Example:

~~~json
{
  "profile": "claude/gateway-a",
  "backend": "claude-code",
  "base_url": "https://gateway.example.com",
  "model": "claude-sonnet",
  "secret_ref": "secret://anthropic/gateway-a",
  "secret_value": "<redacted>"
}
~~~

This is important for debugging mismatched runtime behavior.

## 17. Profile changes and running sessions

Changing a RuntimeProfile must not silently alter already-running sessions.

A WorkSession should retain enough resolved-profile metadata for audit and recovery.

If the owner wants new settings, that should be an explicit session restart/migration where required.

## 18. Configuration ownership

Configuration may eventually be split conceptually:

~~~text
config/
  lain.toml
  profiles.toml
  workspaces.toml
~~~

A database-backed editable registry is also acceptable.

The important requirements are stable IDs, deterministic resolution, inspectability, and clear secret separation.

## 19. First implementation

V1 should support:

- named profiles;
- at least two Codex profiles;
- model/base-url/provider overrides supported by the chosen Codex driver;
- SecretRefs;
- profile validation;
- per-session profile choice;
- redacted inspection.

Do not implement automatic smart routing before explicit profiles are reliable.

## 20. Security rules

- never let model-generated text create arbitrary environment variables without validation;
- prefer structured fields or allowlists for sensitive process options;
- never expose secret values to LainActor;
- never log the resolved worker environment;
- do not blindly inherit the daemon's full environment;
- validate executable selection and workspace paths.

## 21. Implemented daemon configuration

The daemon resolves built-in defaults followed by explicit environment overrides.
It does not load Lain TOML or import/rewrite global worker configuration. Phase 5
adds the SQLite RuntimeProfile registry and a namespaced environment Secret Store.
Daemon settings remain:

| Variable | Default / requirement |
|---|---|
| `LAIN_DATABASE_PATH` | `data/lain.sqlite3`, relative to daemon working directory; parent directories created |
| `LAIN_LISTEN` | `127.0.0.1:8787`; numeric loopback IPv4/IPv6 socket address only; port 0 supported |
| `LAIN_API_TOKEN` | Required, 32–256 ASCII letters/digits/`-`/`_`; use a securely generated random value |
| `LAIN_LOG` | `info`; one tracing level (`error`, `warn`, `info`, `debug`, `trace`) |
| `LAIN_CODEX_PATH` | Optional native Codex executable; otherwise search PATH, including the Windows npm package's native binary |
| `LAIN_WORKER_IDLE_TIMEOUT` | `300`; integer seconds, 1–86400; only eligible idle runtimes detach |
| `LAIN_WEB_DIR` | Optional built Web asset directory containing `index.html`; absent keeps headless serving |

The API credential is resolved separately from domain configuration, held in a
redacted, non-serializable type, and compared using `subtle` constant-time byte
comparison for equal-length values. It is never persisted, emitted in events,
or included in configuration errors/debug output. Supply it through the process
environment; do not put raw credentials in ordinary checked-in files. Its
deployment-level secret backend and rotation flow remain future work. All HTTP
API routes require `Authorization: Bearer <token>`; optional static Web assets are public. Query parameters are not accepted
as credentials. Responses use `Cache-Control: no-store`.

The daemon does not log request headers, request URLs, configuration values, or
environment dumps. JSON lifecycle logs go to stderr and include session IDs/state
and failure context, not session payloads. Configure retention externally.
Use a stable absolute database path for a service; changing its working directory
with the default relative path selects a different database.

## 22. Phase 2 worker configuration

Session creation accepts `options.model` (optional validated model identifier) and
`options.sandbox` (`read_only`, the default, or `workspace_write`). These selected
options persist with the provider binding and are reused on resume. Workspace
must already exist. On Windows configure native `codex.exe`, not a `.cmd`/`.ps1` shim.
This was the Phase 2–4 configuration surface. Phase 5 adds profile selection and
resolved snapshots. Existing unspecified upstream defaults cannot be recovered
retroactively; model/effort are captured on the first successful reattachment.
Lain does not edit global Codex files.

The worker environment is cleared then populated from a small OS/path/locale,
proxy and optional `CODEX_HOME` allowlist. `LAIN_API_TOKEN`, arbitrary credentials
and the rest of the daemon environment are not inherited. Phase 5 adds exactly
one referenced child credential for a managed Responses provider; local profiles
can continue using Codex's existing ChatGPT login. Approval policy is owner
review on request and never automatically accepts a backend permission request.

Raw protocol messages, stderr and provider error strings are not logged. Known
daemon credentials and recognizable credential/key patterns are blocked in
prompts and redacted from completed output/approval descriptions. Fragment events
carry counts, not raw text, to avoid split-token disclosure. This is a sanitizer,
not a general secret detector: arbitrary unknown secret values or encoded secrets
cannot be recognized reliably. Never ask a worker to retrieve/display credentials;
keep credential files outside its permitted workspace. No transcripts or prompts
are persisted as raw transcripts by Lain; Phase 3 stores bounded, sanitized
completed observations in operational history. No semantic memory exists.

Phase 6 stores owner/Actor conversation records separately. Phase 6.6 also stores
the exact non-secret confirmed worker instruction with its delegation link and
dispatch receipt; this is an explicit audit record, not a raw provider transcript.
Existing input checks and namespaced-credential redaction still apply. Result
notices contain bounded factual excerpts of normalized operational history.

## 23. Terminal client configuration

`lain` reads `LAIN_URL` (default `http://127.0.0.1:8787`) and the same required
`LAIN_API_TOKEN`. HTTP and HTTPS URLs may include a reverse-proxy path prefix,
but cannot include user credentials, query or fragment. Authentication uses only
the bearer header, redirects are disabled, and errors do not print request URLs
or credentials. The client has bounded connection/read/request waits and reconnects
SSE after stream loss. Tokens are neither command arguments nor normal output.
Worker workspace arguments are sent verbatim to the server, without local path
canonicalization. Use an existing absolute path on the daemon host; shell tilde
expansion belongs to that shell, not a remote server-path resolution feature.
Daemon binding remains loopback; remote transport/TLS deployment remains future work.

## 24. Web client configuration

The browser uses same-origin API paths and page-memory-only bearer authentication.
`LAIN_WEB_DIR` is a daemon-only, opt-in static directory, validated at startup.
The development Vite process optionally reads `LAIN_DEV_KERNEL_URL`; it never reads
or injects `LAIN_API_TOKEN`. No browser credentials are persisted by default.
Deployment/authentication limitations are in [WEB.md](WEB.md) and ADR 002.

## 25. Implemented Phase 5 profiles

Migration 004 stores `runtime_profiles(id, snapshot)` and
`workspace_profile_policies(workspace, snapshot)` alongside the existing session,
worker and history tables. The Kernel owns all writes. Profiles are definitions
plus a revision, starting at 1 and incrementing on an accepted owner replacement.
PUT requires `expected_revision`, rejecting stale updates/duplicates with 409.
There is no hard deletion; set `enabled: false` to stop new use. Existing attached
or detached sessions keep their snapshot even if the definition changes/disables.

Current supported definition example (owner POST JSON, no secret value):

```json
{
  "id": "codex/gateway-a",
  "backend": "codex",
  "driver": "app-server",
  "enabled": true,
  "model": "your-supported-model",
  "provider": "gateway-a",
  "base_url": "https://gateway.example.com/v1",
  "reasoning_effort": "high",
  "sandbox_default": "read_only",
  "approval_policy": "on_request",
  "secret_ref": "secret://env/gateway-a"
}
```

Only Codex/app-server is implemented. Provider profiles require an explicit
model and SecretRef. Omit base_url for the official `https://api.openai.com/v1`
Responses endpoint. A completely local definition omits provider, base_url and
SecretRef together. It retains existing Codex authentication and endpoint defaults.
Optional reasoning accepts none/minimal/low/medium/high/xhigh/max/ultra; actual
model acceptance is upstream-owned, not guaranteed by syntactic validation.
Approval choices are on_request and untrusted, always with owner review; never,
auto-review, unrestricted sandbox and arbitrary backend options are not offered.
Sandbox choices are read_only and workspace_write. Model and sandbox are the only
advertised per-session overrides. URLs require HTTPS except numeric loopback or
localhost HTTP for local development, and reject credentials, query, fragment,
whitespace/control characters, backslashes and overlong values.

Owner profile APIs may explicitly pair an endpoint and a namespaced credential.
There is no model-generated configuration path or arbitrary daemon env lookup.
No paid/provider request occurs on registry list, validation, UI open or health
inspection. `valid` means local prerequisites and translatability, not verified
authentication, model entitlement or quota. Missing credentials, disabled profiles
and unavailable executables have safe `unavailable` explanations. Unsupported or
invalid definitions are rejected at write time. Invalid/unknown health vocabulary
is retained for future adapters; this implementation does not invent remote health.

## 26. Selection, resolution and frozen sessions

No independent backend selector exists: the profile determines backend/driver.
Creation uses an explicit profile ID, otherwise the canonical workspace policy's
default, otherwise the built-in `codex/local` default. A disallowed/absent profile
fails explicitly; no fallback occurs. The compatibility profile starts with
verified local Codex, no endpoint override or raw credential, read-only sandbox
and on-request owner approvals. Legacy sessions use a revision-0 snapshot of their
previous selected options, preserving all public IDs, native bindings and history.

Field precedence is built-in safe defaults < selected RuntimeProfile < explicit
allowed model/sandbox overrides. Workspace policy selects/restricts profiles;
it does not override model values in Phase 5. Daemon executable/process settings
remain outside the profile. Selection, revision and initial non-secret snapshot
are committed atomically with session creation. A structured initial thread
response fills unspecified local model/effort before work is returned ready.

The persisted private `worker_state.resolved_profile` contains profile_id,
revision, backend, driver, model, provider, base_url, reasoning_effort, sandbox,
approval_policy and secret_ref. It contains no credential value. Registry edits
never rewrite it or reconfigure/restart workers. Idle detach, explicit resume,
lazy follow-up and daemon recovery translate the same snapshot. The current
registry view can have revision 2 while an old session still shows revision 1.
Unmanaged Codex local authentication, local endpoint defaults, tools and other
global CLI settings remain externally owned; this is not a full Codex-home copy.
Use an explicit managed endpoint profile to freeze provider endpoint selection.

## 27. Namespaced Secret Store and redaction

`secret://env/gateway-a` maps only to `LAIN_SECRET_GATEWAY_A`. Names are 1–64
lowercase ASCII letters/digits in nonempty hyphen-separated segments. They cannot
name PATH, LAIN_API_TOKEN or arbitrary daemon variables. Missing, empty, non-UTF8,
overlong or control-bearing credentials fail safely. API views return only the
handle and `available`, `missing` or `not_required`; no secret_value field exists.

The adapter resolves a reference for each new runtime, keeps the child env cleared
and allowlisted, then injects its required value as `LAIN_PROFILE_CREDENTIAL`.
Codex reads that provider env_key; a shell-environment exclusion prevents ordinary
worker commands from inheriting it. No value in argv, profile/snapshot rows,
prompts, API responses, CLI JSON, browser stores, TUI, normal logs or event history.
Redaction includes daemon-known API/namespaced values plus the runtime's resolved
credential. Completed output/commands/paths/tool names and approval descriptions
are sanitized before bounds, persistence and SSE. Diagnostics use static safe
messages derived from structured categories. Buffers held by launch/redaction
use zeroize and clear on drop; the deployment's source environment remains the
Secret Store. This is process configuration isolation, not an OS secret vault.

Profile configuration freeze is not secret value freeze. The same handle resolves
again when detached work attaches. Environment-backed rotation requires restarting
the daemon with the new environment, preserving the database and references; it
does not edit registry definitions or session identity. Never put credentials in
normal CLI arguments or profile JSON. Configure the daemon environment through
the owner's deployment secret mechanism. No global worker auth/config is rewritten.

## 28. Workspace profile policy

The owner PUTs a daemon-host workspace, default_profile and nonempty
allowed_profiles (up to 64). The Kernel canonicalizes the existing directory and
requires known profiles and a default included in allowed_profiles. Policies are
stored in SQLite, not automatically read from worker-editable repository files.
They apply at creation and subsequent prompt/attach/resume, including attached
workers. A newly forbidden profile's history remains readable; accepted work is
not retroactively terminated, and cancellation/approval cleanup remains available.
There is no policy language, automatic routing or workspace-setting overlay.
See [the APIs and commands](DEVELOPMENT.md#14-profile-and-workspace-owner-api).

## Phase 6 local client and independent Actor configuration

Local `lain setup --local` creates private per-user `client.json` and a separate
OS-random `kernel-token`, outside Git. Repeated setup reuses validated credentials.
Windows uses current-owner ACLs; macOS/Linux use owner-owned 0700 directories and
0600 ordinary credential files. `LAIN_LOCAL_DIR` is an explicit absolute override.
`LAIN_API_TOKEN` and explicit remote `LAIN_URL` remain supported; local credentials
are never silently forwarded to a different URL. `lain up` starts the existing
laind beside the CLI (or `LAIN_DAEMON_PATH`) with its saved absolute database and
loopback listener. It refuses remote/listener/database environment overrides;
manual daemon deployment can still use the existing environment-only workflow.

`LAIN_ACTOR_CONFIG` selects non-secret JSON containing openai-responses driver,
explicit model/endpoint/SecretRef, optional reasoning, timeout/output/token bounds.
`LAIN_PERSONA_PATH` optionally overrides the compiled versioned persona at startup.
Actor model configuration is not a Codex RuntimeProfile, and Codex login never
implicitly supplies direct model credentials. Namespaced LAIN_SECRET values stay
in the existing runtime Secret Store, with restart-based rotation. No paid calls
from diagnostics. See [CLI.md](CLI.md) for paths/permissions/overrides and
[LAIN_ACTOR.md](LAIN_ACTOR.md) for the exact model schema and limits.

Phase 6.5 adds explicit saved launcher selections: `lain config actor set --file`
records a non-secret Actor JSON path; `config actor env-file --file` records an
owner-only credential-source path, outside Git. Settings are atomically replaced
under the existing setup lock; no selected file or existing ignored Actor JSON is
rewritten. `config actor [show]` exposes paths/sources/non-secret revisions and
credential availability only. `clear` deselects files without deleting them.

`lain up [--env-file <path>]` loads the explicit process environment over the
selected env file over saved non-secret settings over defaults. The file parser
accepts only unquoted literal UTF-8 `KEY=VALUE`, LF/CRLF and whole-line comments;
allowed keys are `LAIN_ACTOR_CONFIG`, `LAIN_PERSONA_PATH`, and valid namespaced
`LAIN_SECRET_*`. No shell evaluation, interpolation, quote removal, arbitrary
environment keys or cwd/workspace discovery. Env-file relative paths anchor to
that file's directory. Limits: 64 KiB, 128 keys, 8192 bytes/value. The source file
and parent require owner-only native permissions/ACLs and cannot be linked or
inside a Git checkout. Credential values remain only in the explicit protected
file and daemon startup environment, never client settings/SQLite/normal output.

Doctor compares non-secret Actor JSON/persona revisions, optionally with the same
`--env-file`, and reports `restart_required`, `matches`, `not_observable` for older
daemons, or `not_compared_remote`. No secret digest or value comparison occurs.
Revision comparison includes JSON formatting changes. Changes apply on the next
explicit daemon start; there is no hot reload. Inspect active workers, gracefully
stop your owned foreground launcher, and restart with the same database. `up`
never stops an existing service and reports open work when configuration differs.
