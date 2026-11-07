# Agent Backend Integration

## Command-output transport fix — 2026-11-05

The Codex transport absorbs only identified command output progress notifications
before the bounded lifecycle queue; final aggregated command output and all
authority-bearing events retain their normal path. This prevents a fast shell's
temporary chunks from disconnecting a Worker while retaining fail-closed handling
of real lifecycle overflow. Fixed closure reasons preserve diagnostic context
without logging provider data. The first real turn remains uncertain and is not
automatically replayed. See [the exact transport contract](backends/CODEX.md#command-output-burst-recovery--2026-11-05).

## Explicit installed Codex alpha contract — 2026-11-05

Managed execution now accepts exactly 0.160.0 and 0.166.0-alpha.5.1; unknown
adjacent alphas remain rejected. For the latter, the installed offline schema's
13 consumed request/response/notification field and enum groups were checked,
and native isolated probes pass initialization, empty thread start/fork and
same-ID resume, plus both existing Rust adapter profile-injection and unused
thread-persistence probes. No inference or personal login was used. Existing
structured fixture lifecycle/permission checks remain distinct from a new real
provider task. This is not semver-range compatibility or paid-run acceptance.
Diagnostics add `supported_versions`; `supported_version` retains the original
baseline for older clients. The human CLI prefers the full explicit list.

## W4 original-session interaction and permission facts — 2026-11-03

Remote follow-up reuses the original protected native Supervisor/provider binding
with a distinct new turn; running steering is not implied. Explicit task-cancel
UI reuses the scoped command API and native effect fences, not screen parsing.
Device permission reports distinguish durable receipt from the matching
Supervisor approval being resolved; completion/transport acknowledgement alone
is insufficient. No uncertain decision/submission is replayed. See
[device work](DEVICE_WORK.md), [permission evidence](DEVICE_EXECUTION.md) and
[W4 gates](W4_ACCEPTANCE.md). No additional backend or real-provider acceptance.

## W3b device reuse of managed Codex — 2026-11-02

The explicit native device service uses the existing managed-control Supervisor,
structured transport and 0.160.0 version gate; it adds no second planning/tool
loop. Deterministic Lain session/turn metadata and device-owned durable fences
prevent replacement sessions after uncertainty. Device/owner credentials remain
outside backend requests and are included in completed-output redaction. Actual
provider lifecycle and native platform acceptance retain their separate scope.
See [ADR 030](adr/030-native-device-worker-service.md) and [DEVICE_HOST.md](DEVICE_HOST.md).

## W1b explicit visible-history reader — 2026-10-27

One authenticated `archive read` command can read a bounded page from a registered
local CodexSession source and atomically store normalized visible records with a
read receipt. Native 0.160.1 paginated reads were verified on isolated synthetic
history. Reader compatibility is separate from the unchanged managed-control
gate at 0.160.0; diagnostics expose `archive_read_version`. Reads never resume
external work, approve requests or create Lain conversation/memory/inference.
There is no autonomous collector. See [ADR 020](adr/020-explicit-visible-worker-archive-reads.md).

## W1a declared visible archive imports — 2026-10-26

The [archive foundation](ARCHIVE_ACCEPTANCE.md) stores bounded owner-imported
visible Codex-session slices with declared installation/workspace/session origin.
That owner-import path does not call Codex, authenticate the declaration or
create a managed WorkSession. The separate W1b read path is described above;
existing Worker control contracts remain unchanged.

## Target archive and device boundary — 2026-10-23

The product now includes archives of both Lain-created and independently created
visible Worker sessions. Reading/importing history is a capability separate from
running, resuming, steering, cancelling or approving a session. An archive must
not fabricate a live WorkSession or control relationship. Explicit takeover must
be supported and verified; do not start a replacement session and call it resume.

Future paired PCs own native execution and report provenance-bearing evidence to
the cloud. The current Supervisor only manages same-host Codex processes.
Native terminal entry is optional and capability-dependent; the unified Lain
interface remains primary. No universal complete-transcript or native-attach API
is assumed. See [RESIDENT_DESIGN.md](RESIDENT_DESIGN.md) and [PRODUCT.md](PRODUCT.md).

The existing adapter requirements and implemented contracts below remain valid.

**Status:** target archive/device update plus existing adapter contracts
**Last reviewed:** 2026-10-23

## 1. Purpose

Lain delegates substantial work to external worker-agent runtimes.

Initial targets:

- Codex;
- Claude Code;
- DeepSeek Harness.

The first implementation should focus on Codex.

## 2. Principle

> Integrate harnesses. Do not reimplement harnesses.

A backend adapter translates:

~~~text
provider/runtime protocol
        |
        v
Lain backend contract
        |
        v
normalized WorkSession / Turn / Event model
~~~

It must not reproduce the provider's reasoning loop.

## 3. Integration hierarchy

Prefer, in order:

1. official structured long-running agent API/server;
2. stable interoperability protocol such as ACP;
3. official SDK through a supervised sidecar;
4. documented machine-readable non-interactive CLI mode;
5. terminal automation only as a last resort.

Interactive terminal scraping is not a foundation because rendering is not a protocol, prompts change, ANSI UI is unstable, provenance is lost, and recovery becomes ambiguous.

## 4. Backend semantic contract

The semantic boundary supports the following concepts; a generic Rust trait is
deferred until a second backend justifies it:

- probe backend health/version;
- report capabilities;
- create session;
- resume session;
- send prompt/follow-up;
- receive structured events;
- cancel active work;
- close/archive session where supported;
- respond to approval;
- shut down owned runtime resources.

Streaming events may use channels/streams rather than callbacks.

## 5. Capability model

Backends are not identical.

Capabilities may include:

~~~text
session_resume
multi_session_runtime
structured_events
approval_requests
structured_tool_events
structured_file_events
usage_reporting
model_selection
reasoning_level
dynamic_config
subagent_visibility
background_commands
~~~

Core behavior should branch on capabilities, not provider names.

Provider-specific cases belong in adapters whenever possible.

## 6. Event normalization

Provider events map into Lain events.

Example:

~~~text
Codex item event
Claude SDK tool event
ACP tool-call update
        |
        v
ToolStarted / ToolUpdated / ToolCompleted
~~~

Normalization should preserve optional provider metadata for debugging.

Do not destroy useful detail just to fit the smallest common denominator.

## 7. Session identity

Lain assigns WorkSessionId.

Provider-native IDs are adapter metadata.

~~~text
Lain:
  work_session_id = work_01...
  backend = codex

adapter:
  provider_session_id = thread_...
~~~

Provider ID must not be the only durable public identity.

## 8. Codex

### Recommended first path

As of 2026-10-04, Codex App Server provides a structured long-lived interface with thread/turn/item concepts, progress events, and approval handling.

That maps naturally to Lain's needs.

The implemented Codex adapter speaks App Server directly. See
[the verified contract and test results](backends/CODEX.md).

### Conceptual mapping

~~~text
Codex thread     -> WorkSession
Codex turn       -> Turn
Codex item       -> normalized Event(s)
approval request -> Approval
~~~

The mapping was verified against installed 0.160.0 on 2026-10-04. Lain persists
opaque provider bindings separately from public WorkSession/Turn IDs.

### Configuration

Phase 5 resolves named profile configuration and freezes non-secret session
snapshots, including selected model/sandbox overrides. The capability report describes the locally
verified version's adapter operations, not remotely negotiated server permissions.
Unknown versions are unavailable until their contract is verified. Session startup
still performs initialization and can fail independently of the version probe.

Prefer per-runtime/session overrides over rewriting global user config.

## 9. ACP

ACP is a preferred interoperability layer where mature support exists.

Benefits include a common session lifecycle, prompt/update flow, approvals, adapters for multiple agents, and Rust SDK support.

Rules:

- ACP stays inside protocol/adapter code;
- protocol version negotiation is adapter responsibility;
- ACP types do not become core domain types;
- native drivers and ACP drivers may coexist;
- absent features are capabilities, not reasons to distort core types.

As of 2026-10-04, ACP v2 is evolving. Re-check upstream docs before coding against it.

## 10. Claude Code

Candidate paths:

1. ACP adapter backed by Claude Agent SDK;
2. direct Agent SDK sidecar;
3. a newer official structured interface if one becomes preferable.

A sidecar is acceptable if the best SDK is not Rust-native.

Implementation should investigate:

- session creation/resume;
- structured tool updates;
- permissions;
- model selection;
- workspace control;
- cancellation;
- subagent/background visibility.

All results still normalize to Lain's WorkSession model.

## 11. DeepSeek Harness

DeepSeek Harness is a useful later backend because it exposes programmatic session control and ACP-related integration.

Add it after Codex is stable.

Its purpose is partly to validate that Lain's abstraction works across independent harnesses rather than only for one provider.

## 12. Runtime supervision

An adapter may require a local process.

Backend Manager should own:

- executable discovery;
- version probe;
- process start;
- protocol framing;
- stderr capture;
- health state;
- exit detection;
- restart policy;
- graceful shutdown.

Frontends must never spawn worker processes directly.

## 13. Version detection

Each adapter should expose:

~~~text
backend kind
driver kind
runtime version
protocol version
capabilities
~~~

Persist relevant version metadata when useful for debugging/recovery.

Version-dependent features should fail explicitly rather than be guessed.

## 14. Error normalization

Preserve normalized category plus provider detail.

Candidate categories:

~~~text
Unavailable
AuthenticationFailed
RateLimited
ProtocolError
Unsupported
WorkspaceError
PermissionDenied
ProcessExited
SessionNotFound
SessionNotResumable
Cancelled
Unknown
~~~

Do not reduce failures to assistant prose at the adapter boundary.

## 15. Approval routing

~~~text
worker
  |
  v
adapter
  |
  v
ApprovalRequest
  |
  +--> persistence
  +--> Web/TUI
  `--> optional Lain explanation
             |
owner decision
             |
             v
ApprovalResponse
             |
             v
adapter -> worker
~~~

Lain may explain risk but should not silently approve dangerous actions unless explicit policy permits it.

## 16. Backend-specific docs

When adapters are implemented, add:

~~~text
docs/backends/CODEX.md
docs/backends/CLAUDE_CODE.md
docs/backends/DEEPSEEK_HARNESS.md
~~~

Each should record:

- last verified date;
- tested version;
- startup/API method;
- protocol version;
- domain mapping;
- limitations;
- config fields;
- recovery behavior;
- test procedure.

Volatile facts belong there, not in the core architecture.

## 17. Adapter acceptance test

A backend is minimally useful when Lain can:

1. detect it;
2. create a session;
3. send a prompt;
4. receive structured progress;
5. determine turn completion/failure;
6. cancel;
7. persist provider session identity;
8. resume where supported;
9. route approvals where supported;
10. survive frontend disconnect;
11. expose normalized state through the same API as other backends.

## 18. Upstream references

Re-verify before implementation:

- Codex App Server: https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- ACP: https://github.com/agentclientprotocol/agent-client-protocol
- ACP Rust SDK: https://github.com/agentclientprotocol/rust-sdk
- Codex ACP adapter: https://github.com/agentclientprotocol/codex-acp
- Claude ACP adapter: https://github.com/agentclientprotocol/claude-agent-acp
- DeepSeek Harness: https://github.com/deepseek-ai/deepseek-harness

## 19. Durable work and temporary runtimes (Phase 3)

Adapters must keep provider bindings distinct from runtime/process ownership.
Idle release and lazy resume retain logical WorkSession identity, without
synthetic model prompts. New unused sessions must be proved resumable before an
adapter advertises safe detach. The current Codex-specific creation materialization
sequence and verified version are documented in [CODEX.md](backends/CODEX.md).
Only uncertain provider-backed sessions reconcile at boot. Connection-local
approvals cannot be transferred to another runtime; planned idle release requires
none unresolved. Planned release/shutdown and unexpected transport loss have
different domain semantics; public runtime observations never expose process IDs.

Normalized completed observations commit to operational history before live
publication. Raw fragments, reasoning, protocol objects and private diagnostics
stay in the adapter and never become history. A terminal client uses these same
provider-neutral APIs; it does not add a second backend implementation.

## 20. Profile configuration and diagnostic evidence (Phase 5)

A profile determines backend/driver; a session does not independently select an
inconsistent backend. The verified Codex adapter translates a resolved profile
into process-scoped settings and a dedicated child credential variable. Concurrent
runtimes do not share mutable configuration. Profile-supported overrides are
advertised in registry views, alongside local health and secret availability.
No generic backend plugin trait or new worker intelligence was introduced.

Structured Codex error discriminants and recognized HTTP status fields map into
Lain-owned failure kinds and optional retryability. Raw diagnostics are discarded.
Unknown evidence remains Unknown. Error observations do not replace final turn
evidence. Credential redaction includes namespaced daemon values and the current
runtime's resolved secret before approval/output/history/SSE. Exact version and
mapping limits remain documented in [CODEX.md](backends/CODEX.md).
