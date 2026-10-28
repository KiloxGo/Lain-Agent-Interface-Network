# W2a explicit device enrollment and inventory

2026-10-28 (Asia/Shanghai). Contract: [ADR 026](adr/026-explicit-device-enrollment.md).
Implemented and locally validated. This is device identity and reported inventory,
not remote command delivery, PC background execution or completion of W2/W3.

## Operation

Use an isolated schema23 Kernel, or a separately authorized stopped complete
backup/upgrade. Existing personal schema16 launch/staging scripts must not install
these binaries. The listener remains loopback; no public Kernel was deployed.
For an actual remote device, separately configure protected HTTPS termination.
Plain HTTP is accepted only on loopback. Device commands disable proxies,
redirects and built-in retries, including their administrator enrollment path.

1. On the selected device, choose one existing native workspace, a new absolute
   credential directory outside Git whose parent already exists, an HTTPS Kernel
   endpoint and a finite Unix-millisecond expiry within 31 days:

   `lain device init --directory <private-dir> --endpoint <https-kernel> --label
   <name> --workspace <existing-path> --expires-at-ms <future-ms>`

   Only the device-local `device-credential.json` contains the OS-random bearer.
   Ordinary output contains paths/IDs and a fingerprint, never that value.
   `registration.json` is a separate document with no bearer credential.
   Init does not contact a Kernel, pair anything or discover/claim Worker support.
   Repeated init refuses to overwrite an existing directory or rotate its key.

2. Transfer and inspect only `registration.json` through a trusted channel. On an
   administrator client, run `lain device pair --file <registration.json>`.
   This admits an exact Pending manifest for at most ten minutes. Do not send the
   administrator bearer to the device. Registration alone starts no work.

3. On the device, run `lain device claim --directory <private-dir>`. The device's
   key and digest of the exact registration must match the administrator-admitted
   manifest. A changed manifest conflicts. Same claim request/key is idempotent;
   expiry/revocation prevents activation. Inspect `device status --directory
   <private-dir>` or administrator `device show <id>`.

4. Make one explicit local inventory report with `lain device report --directory
   <private-dir> --sequence <next-sequence> [--request-id <uuid>]`. The native CLI
   reports only declared workspace paths still present at the same canonical
   location; it claims no Worker capability without a native probe. Server
   receive time and device-reported time are separate. Sequence gaps and changed
   duplicate requests conflict. Inspect status/receipt before another attempt
   after uncertain HTTP; no hidden retry or standing heartbeat is installed.

5. Administrators use `lain device list`, `device show <id>` and `device revoke
   <id> --revision <current-lifecycle-revision>`. Reports do not change that
   lifecycle revision. Revocation blocks old proof/report retries and cannot be
   undone by re-sending an old registration. Replacement is a newly reviewed
   identity/key. Last-report time is not an Online guarantee.

Administrator API: GET/POST `/v1/devices`, GET `/v1/devices/{id}`, POST
`/v1/devices/{id}/revoke`. Separate device API: GET `/v1/device-agent/{id}`, POST
`/v1/device-agent/{id}/claim` and POST `/v1/device-agent/{id}/reports`. A device key
does not authenticate administrator, Actor, chat, memory or Worker APIs. The
administrator key and public credential fingerprint are not device proof.

The Kernel opens no reported device path. Declared capabilities and reported
versions remain device claims and create no runtime compatibility or action
authorization. Existing Worker version/approval checks remain intact. There is
no command API, external-session takeover or second device persona in this slice.

## Validation

Native Windows/MSVC: **267 Rust tests pass, 0 fail, 3 manual ignored**. Format,
strict workspace/all-target Clippy, locked native build and all **14 Python
script tests** pass. Thirteen new domain/API/client/config tests plus extended
streaming checks cover the device boundary, including the reserved credential
namespace and all-split redaction of recognizable device keys.

Six Store tests pass for explicit/default-off registration, exact request/claim
idempotence, credential-verifier separation, monotonic/restart receipts, expiry,
revocation, forbidden inventory, atomic failure, full report ledgers and
byte-preserving schema22 migration. Three HTTP tests pass for separate principals,
malformed/sensitive input without echo/persistence and revoke while a report body
is delayed. Three native Windows client tests pass for private files, no overwrite,
registration mismatch, Git-location and cleartext-transport refusal. The existing
streaming redactor test now includes device credentials, and an additional
configuration test rejects them as Kernel administrator tokens.

`scripts/device_pairing_smoke.py` exercises separate owner and device CLI paths
against a native Kernel with no owner credential in the device process, actual
workspace availability, restart receipts and revocation. Its fixture does not
install a real agent or call a model. It passes **5 checks / 3 durable reports /
0 model requests**, including a local manifest changed after administrator
admission being refused, no inherited owner credential in the device process,
missing workspace honesty, restart and no key in database/logs/ordinary output.

Actual native schema22→23 rehearsal passes **5 checks / 3 fake model requests**:
complete stopped ACL-preserving backup, original records/confirmed deliveries/
memory/private-chat bytes preserved, no invented device or source-memory grant,
old-binary refusal and complete old-state/binary rollback. Full chat regression
passes **36 checks / 83 fake requests**; source-memory regression passes **5 / 16**;
source bridge/sync passes **6 checks / 10 GETs / 10 jobs**, with zero model calls.
Final reports match the same three binary hashes and confirm temporary cleanup.

Initial compile failure (SQLite integer conversion), Clippy's needless-reference
finding and the local log-overwrite guard refusal are retained rather than counted
as passes. Initial W2a logs are ignored under local `target/w1f/w2a-*`; final logs
and report aggregation are under `target/w2a`, including `workspace-final.log`,
`clippy-final.log`, `device-native.log`, `upgrade22.log`, `process.log`, `memory.log`
and `bridge.log`. No paid call, real QQ send, actual device installation or
personal schema migration was initiated by this implementation.

Cross-device HTTPS deployment, native
macOS/Linux, command/event resync, device runtime lifecycle and W3/W4 remain open.
