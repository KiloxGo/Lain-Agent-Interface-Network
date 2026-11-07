# ADR 026: Explicit device enrollment and reported inventory

Date: 2026-10-28. Status: accepted W2a implementation contract; locally validated.

## W2b additive extension — 2026-11-02

[ADR 028](028-durable-device-commands.md) extends the same principal with its own
command/receipt/change routes. This adds no Worker, Actor or administrator access.
The W2a-only capability statements below describe the original increment.

## Scope

W2a establishes a revocable device identity and reported inventory. It creates no
remote command, Worker, archive collector or additional Lain persona. W2b command/
receipt/resynchronization and W3 native execution remain separate increments.
No personal/cloud Kernel migration, installation or public listener is implied.

## Enrollment

The device generates an OS-random 256-bit `lain-device-` credential in an
owner-only directory outside Git. A separate non-secret registration document
contains its domain-separated SHA-256 fingerprint, Lain DeviceId/WorkspaceIds,
native platform, label, exact canonical device workspace declarations and finite
expiry. Transfer/review this registration document using an already trusted
channel; do not distribute the Kernel administrator credential to the device.

An authenticated administrator explicitly admits that exact registration under a
request ID. It is Pending for at most ten minutes. The new device proves knowledge
of its credential on a separate claim endpoint before the registration becomes
Active. Claim also binds a digest of the exact local registration, so a changed
manifest in transit cannot silently become the device's selected scope.
Repeating the same admitted registration/claim is idempotent; changed
payloads conflict. A verifier cannot be used as a bearer credential. Reusing a
credential fingerprint for a different device is rejected.

Pending/Active/Revoked lifecycle revisions belong to the Kernel. Expired/revoked
devices cannot claim or report, and revocation cannot be undone by re-enrollment
of the same identity. Replacement uses a newly reviewed identity/credential.
Authorization expires within 31 days. Old identities/receipts remain operational
evidence; expiry is enforced on reads/writes without an idle inference loop.

## Authentication and transport

Administrator and device routes have distinct authentication. Device credentials
use a reserved prefix that Kernel administrator configuration rejects. Device bearer
credentials permit only claim, own metadata and bounded inventory reports for the
selected DeviceId. They cannot read chat/memory, change grants, manage Workers or
authenticate as another device. The administrator bearer is not a device proof.
Every operation rechecks credential/state/expiry under the storage lock used by
revocation; a pre-authenticated request cannot commit after revocation.

Kernel listener remains loopback. The client accepts HTTPS, with HTTP restricted
to numeric/localhost loopback fixtures/development. Remote use requires separately
configured authenticated TLS termination; no trust in forwarded headers or
automatic public exposure is added. Redirects, proxies and built-in retries are
disabled for the device client. Credentials are never command-line arguments,
normal CLI output, API metadata, prompts, events or normal logs. The recognizable
credential prefix participates in complete-record and streaming redaction.

## Reports and bounds

An immutable owner-admitted manifest declares workspace IDs/paths and selected
capability kinds. Paths are device-platform strings, never canonicalized or
opened by the cloud Kernel. A report can only list a subset of those workspaces
and capabilities; versions/availability are device claims, not independently
verified runtime support or action authorization. The native CLI checks its own
declared workspace paths before reporting availability. It does not claim a
Worker version without a native probe.

Reports have per-device monotonic sequence, request ID, original reported time
and Kernel receive time. Same-ID/same-payload replay returns its durable receipt;
changed IDs/payloads or sequence gaps conflict without advancing state. Failed or
uncertain HTTP requests are inspected before another attempt; there is no hidden
report/command retry or background poller in W2a. Last contact is a timestamp,
not proof a device remains online. Lifecycle revision is independent of report
sequence, so reporting cannot prevent a revision-checked administrative revoke.

Limits: 128 registered identities, eight workspaces, four capability kinds,
4096 report receipts per identity, bounded UTF-8 metadata and API bodies. Filling
a report ledger does not prevent revocation. Migration 023 enables no device and
preserves old state. Credentials are indirect device-local protected material;
Kernel storage contains only a high-entropy credential verifier, never the key.

## Acceptance

Test isolation/default-off, proof without owner-key sharing, duplicate enrollment
and reports, wrong keys/device IDs, expiry/revoke races, monotonic receipt restart,
native protected file creation/permissions/no overwrite, secret rejection and
byte-preserving schema22 upgrade. Actual cross-device TLS deployment and native
Linux/macOS are separate from two native Windows loopback processes. No test may
claim remote execution, persistence across device shutdown or completed W2/W3.
