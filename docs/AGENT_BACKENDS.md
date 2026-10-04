# Agent Backend Integration

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

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

The exact Rust trait is deferred, but the adapter layer should support the semantics of:

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

The first Codex adapter may speak App Server directly.

### Conceptual mapping

~~~text
Codex thread     -> WorkSession
Codex turn       -> Turn
Codex item       -> normalized Event(s)
approval request -> Approval
~~~

The exact mapping must be verified against the installed Codex version during implementation.

### Configuration

Resolve RuntimeProfile into launch/session configuration.

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
