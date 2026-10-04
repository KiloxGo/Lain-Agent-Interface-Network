# Configuration and Runtime Profiles

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. Purpose

Lain should launch worker agents with different providers, API endpoints, models, policies, and credentials without forcing the owner to manually edit each worker's global configuration.

This document defines the profile model.

## 2. Principle

> Configuration belongs to named runtime profiles. Secrets belong to a secret store.

A profile describes how to start or control a worker. It does not contain raw secret values.

## 3. RuntimeProfile

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
