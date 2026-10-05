# ADR 003: Mutable profiles, frozen session configuration, runtime SecretRefs

Date: 2026-10-05. Status: accepted; Phase 5.

## Decision

The Kernel owns a SQLite Profile Registry. Definitions use stable names and
monotonic revisions. Owner bearer APIs create profiles and replace definitions
with an expected revision. Profiles have structured fields, not arbitrary argv,
environment maps or worker-native JSON. Disabling prevents new sessions; existing
snapshots remain usable. There is no cascade deletion or live reconfiguration.

Creation resolves defaults, profile and allowed model/sandbox overrides under the
Kernel storage lock, selects the canonical workspace's default/allowed policy,
and atomically persists WorkSession plus non-secret resolved worker metadata.
Backend/driver follow the profile. Registry edits never update this snapshot,
restart processes or change historical events. Local model/effort defaults are
filled from the initial structured thread response. Resume translates the same
snapshot and keeps the existing provider binding.

SecretRef supports only `secret://env/<lowercase-hyphenated-name>`, mapping to
`LAIN_SECRET_<UPPERCASE_UNDERSCORED_NAME>`. No arbitrary environment lookup is
available. The environment belongs to the daemon's deployment, not a profile.
Rotation of an environment-backed value requires starting the daemon with a new
environment; references resolve again at each runtime attachment. Definition
freeze is separate from credential-value freeze. No raw value is serializable in
a profile, worker snapshot, API view or history.

The Codex adapter translates a validated resolved profile into its private launch
spec. The child environment remains cleared and allowlisted, with only its own
required provider credential added. Secrets are absent from argv. Managed Responses
providers use an adapter-owned configuration name and explicit endpoint/auth
configuration; local authentication continues using existing Codex local state.
Secret-bearing buffers and runtime redaction copies clear on drop. Redaction knows
the daemon token and namespaced daemon credentials plus the current runtime's
resolved credential, before any durable or public output.

Workspace policy is owner-managed in SQLite, never imported from worker-editable
workspace files. It restricts new selection and subsequent prompt/attach/resume,
including already attached runtimes. It does not retroactively stop accepted work
or hide history. Cancel/approval cleanup remains available.

Migration 004 seeds `codex/local` and adds revision-0 compatibility snapshots to
old local worker metadata without changing public/provider IDs, turns, approvals
or history. Old unspecified upstream defaults cannot be reconstructed; the first
reattachment captures safe model/effort evidence. Codex's existing local auth,
unmanaged tool settings and local endpoint defaults remain externally owned.
New managed endpoint profiles must specify model, provider and SecretRef.

## Consequences

No global provider switch or config rewrite exists, and simultaneous runtimes use
independent settings. Read-only health checks validate schema, executable and
credential availability without inference or provider probes; valid means local
prerequisites, not verified account access. The version gate remains 0.160.0.
Only Responses gateways are supported. A future secret backend or second worker
family can justify new machinery; this milestone adds neither a plugin framework
nor an Actor, secret editor, external auth probe, Vault or encryption scheme.
