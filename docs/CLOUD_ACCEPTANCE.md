# Cloud continuity acceptance — 2026-11-05

Later explicit QQ cutover: the server-hosted Lain account is logged in and Online,
the old PC reply runtime is quiesced without deleting history, and real private
text completion/send/user-receipt evidence passes. A further idle Kernel restart
preserves the same binding/conversation/reply grant and original database/model/
budget. The user separately enabled a seven-day private-binding memory trial;
actual remember/correct commits and current/superseded records pass owner review.
This is not cross-channel memory sharing, image input, live forgetting or multi-day
acceptance. One early real turn ended Unknown without returned usage and was not
replayed; subsequent ordinary and memory turns completed. Its specific cause is
not established by the retained public receipt. See [QQ_CHANNEL](QQ_CHANNEL.md).

Subsequent explicit cloud-chat activation on the same date configured DeepSeek
Flash and completed one short real conversation, using one provider request.
The separate native Windows cloud entry is described in [CLI](CLI.md#cloud-chat-entry--2026-11-05).
Personal data, QQ, device execution and broader real-provider/multi-day acceptance
remain separate. The empty/unconfigured state described below is the original
bounded continuity-stage handoff, not the current Actor configuration.

This records the bounded cloud-residency, scoped-memory and multi-device stage.
It is not real-provider, QQ or long-term daily-companionship acceptance. The
deployment uses an empty cloud instance; personal data and the existing daily
client configuration were not migrated. Source is an uncommitted development
snapshot, not a tagged release or a Git commit.

## Delivered behavior and evidence

| Original requirement | Implemented / deployed behavior | Verification and limits |
| --- | --- | --- |
| 1. Restore facts and preserve boundaries | Existing workspace, cloud identity, private configuration and applications retained; operational artifacts outside Git | Initial/final inventories, manifests, exact process hashes and unchanged configuration/database inode. No reset, personal DB import, commit or push |
| 2. One cloud identity and local execution | Kernel owns Actor, policy, conversations and semantic state; native device service owns supervised local Codex execution | Actual HTTPS client/Worker runs; closing frontend leaves work independent. Codex owns planning/tools/context; channels remain adapters |
| 3. Maintainable cloud service | Native Rust, SQLite, low-privilege systemd, loopback Kernel behind existing HTTPS proxy, private state, finite logs/releases/backups | Actual certificate/hostname/auth/SSE, service restart, native backup regressions, isolated restore and database rollback rehearsal. Current and previous binaries both use schema32 |
| 4. Cloud memory with scope isolation | Stable project/device/workspace IDs, explicit mappings, independent origin/visibility, Kernel-selected strict model tools and checked lineage/publication | Native domain/HTTP/Actor regressions plus seven actual HTTPS memory cases. Equal paths are synthetic registrations on one Windows PC, not two physical PCs. Real QQ is not exercised |
| 5. Startup and continuous synchronization | Versioned bounded bootstrap, persistent identity/boot generation, content-free invalidation, reconnect/resnapshot, monotonic leases and private metadata cache | Gap/cursor/deadline/late-response/auth/fault tests and actual HTTPS terminal changes, scroll/reopen/source withdrawal. No semantic/transcript offline cache or second local persona |
| 6. Actual Linux–Windows work chain | Explicit pairing/reporting, dispatch, exact permission handling, follow-up, cancellation, independent Chat/Work, durable receipts and unknown-state fences | Twelve actual HTTPS structured Worker cases on the deployed executable; exact accept/decline values independently observed. No paid model used |
| 7. Authorized execution and local validation | Scoped deterministic fixtures, small regressions, native builds/checks, documented changes and finite cleanup | Private evidence retains failures, reviews, fixes, hashes and stopped jobs. No real channel login/send/listening, new inference spend, personal migration, DNS/account changes or other-app cleanup |
| 8. Operational handoff and explicit limits | Maintained architecture, lifecycle/rollback/removal guide, resource inventory, reproduction paths and private evidence index | Final audit binds current process, tested binaries, source manifests and cleaned fixtures; real use remains separate below |

The architecture diagram and ownership contract are in
[CLOUD_CONTINUITY](CLOUD_CONTINUITY.md). Operational limits, measured resources,
upgrade/recovery and non-destructive removal are in
[CLOUD_OPERATIONS](CLOUD_OPERATIONS.md). Scope and cache details are in
[MEMORY](MEMORY.md), [ADR034](adr/034-contextual-semantic-memory.md),
[CLIENT_SYNC](CLIENT_SYNC.md) and
[ADR035](adr/035-client-startup-and-live-invalidation.md).

## Verification provenance

The final compiled Rust/identity snapshot has 228 inputs. It differs from the
prior full native Linux workspace check in five inputs: three Actor tool/test
files, the structured test App Server and the native Windows private-path helper.
Client synchronization, migrations, wire contracts and Linux local-file logic
are unchanged. Validation is attributed to its actual snapshot rather than
adding repeated test totals together:

- The prior full native Linux workspace passes 429 tests, three existing manual
  tests ignored. Its native Windows runtime consumer check passes 89 selected
  tests, one existing manual ignored, with strict workspace/all-target Clippy
  and formatting. Ten native ConPTY interaction cases pass.
- The final source passes 328 selected native Linux tests, two existing
  manual/live ignored: Kernel280, Actor10, reliability7, foundation11, memory9,
  profiles5 and wire6. Windows final Kernel280 and Actor/reliability17 pass;
  native local-file6 and terminal84 checks cover the Windows path repair.
  Targeted strict Windows Clippy and formatting pass.
- Linux Clippy was not rerun for these increments. The initial Rust1.89 bootstrap
  used one explicitly recorded `nonminimal_bool` style exception; there is no
  claim of a new exception-free strict Linux lint pass. Native macOS remains
  unverified.
- Seven strengthened actual HTTPS memory cases use 38 bounded local fake-model
  calls, exact retrieved IDs/revisions/scopes and separately captured model
  inputs. Six assertion guards and five model-contract tests pass. Review fixes
  strengthen fresh-active deletion/revocation, inferred provenance, strict nested
  schemas and all-page terminal inspection.
- The latest twelve HTTPS Worker cases use the same Linux executable and native
  Windows client as memory acceptance. Twenty provider threads comprise ten
  empty initialization seeds and ten durable work threads; twelve turns are
  actually accepted. Recovery adds no task submission. This measures normalized
  sessions/threads/turns, not the number of operating-system process spawns.
- Worker faults include Kernel and native-host restart, lost native follow-up
  receipts, lost committed HTTP responses and real network partition. Two unique
  post-fault server facts appear in the same real terminal; its unsent draft
  remains unsent. Owner, device and local model-provider credentials stay separate.
- Backup tests pass nine native Linux cases, including closed/live/crash WAL,
  link refusal, private restore, finite aggregate bytes and preserving the last
  good archive on failure. An isolated mutation removing the byte guard fails.
  Eight initial-upgrade rollback guards, three same-schema guards and two exact
  obsolete-release layout guards pass separately.

The private evidence directory retains the original eight-part goal, historical
increment reports, final requirement-by-requirement audit, source/build hashes,
native logs, raw synthetic model inputs, screenshots, operation receipts and
cleanup archive. It is intentionally not committed or copied to shared storage.
Historical reports describe their original state; the final audit supersedes
their then-pending deployment/acceptance fields without rewriting history.

## Reproduce without personal data or paid calls

Use the repository's [native development commands](CLI.md) and actual installed
toolchains; Windows and Linux are separate native checks. Preserve the current
workspace. Core targeted entries are:

```text
cargo fmt --check
cargo test --locked -p lain-kernel --lib
cargo test --locked -p lain-kernel --test actor --test actor_reliability
cargo test --locked -p lain-client
cargo test --locked -p lain-local
cargo clippy --locked -p lain-kernel -p lain-client -p lain-local --all-targets -- -D warnings
```

On Linux, run the backup helper's isolated tests as documented in
[operations](CLOUD_OPERATIONS.md). The memory assertion and model-contract
tests use only Python's standard library:

```text
python -B -m unittest discover -s scripts -p test_cloud_memory_smoke.py -v
python -B -m unittest discover -s scripts -p test_cloud_fixture_server.py -v
```

Actual HTTPS scripts are [cloud_worker_smoke.py](../scripts/cloud_worker_smoke.py)
and [cloud_memory_smoke.py](../scripts/cloud_memory_smoke.py). They require native
Windows ConPTY dependencies, explicitly prepared synthetic cloud fixtures, matched
built binaries, an outside-checkout artifact directory and the selected portrait
file. A private operator bridge supplies `LAIN_CLOUD_FIXTURE_URL` and the disposable
`LAIN_CLOUD_FIXTURE_OWNER_TOKEN` through the child environment; memory acceptance
also pins `LAIN_CLOUD_FIXTURE_BINARY_SHA256`. Do not save those values in source,
command arguments or ordinary logs. A running
production Kernel alone is not a fixture. Prepare a separate authenticated
temporary route/service/state using the private operator's inspected manifest;
verify source/binary hashes before invoking either script. Each script accepts
`--bin-dir`, `--artifacts` and optional `--portrait-file`. Do not point them at a
personal instance or infer authority to create another public deployment from
these reproduction notes. Stop/remove the exact temporary fixture afterward and
verify the production process and other applications remain unchanged.

## Entering actual daily use

The CLI is a client of its configured Kernel. Cloud mode connects to the cloud;
development `lain up` still starts a local development Kernel. This stage does
not silently switch the saved daily endpoint. Cold cloud startup requires a
reachable Kernel. The native device service must also be configured and started
explicitly; merely opening the TUI or enrolling a device does not start Codex.

The original handoff's next steps began with explicit cloud Actor model/SecretRef
configuration and client endpoint selection; those two are now completed by the
narrow chat activation noted above. Device enrollment and a bounded real-provider
Worker task acceptance remain. QQ daily companionship additionally
needs its independently configured adapter/account, selected bindings and grants,
and real login/send/recall/correction/proactive-delivery acceptance. Define those
effects and inference limits before enabling them. Existing API keys or old QQ
configuration are not reused as new authorization. Offsite disaster recovery,
native macOS and multi-day reliability/naturalness remain unverified.

There is no measured latency comparison against direct Codex yet. Dispatch,
status and permissions add a cloud hop; Codex's local execution loop and provider
connection remain its own. Conversational planning through the Actor may add a
model round, while explicit Work dispatch does not require that conversation.

### 2026-11-05 multi-step chat repair

User terminal feedback reproduced a literal memory-query miss followed by a
cumulative per-turn token failure. The saved memory remained active. The repair
retains all returned usage and completed receipts, distinguishes known token
exhaustion from uncertain execution, describes literal filtered retrieval to the
Actor and respects Windows Terminal fullscreen by using Ctrl-J for newline.
Historical messages and memory provenance are unchanged. Plain Markdown rendering
and a provider's interpretation of evidence remain outside these deterministic
claims.

Native Windows validation passes 282 Kernel library tests (two manual probes
ignored), 86 client tests (one ignored), one CLI integration, formatting and
strict affected-target Clippy. Ten native ConPTY checks pass using five simulated
model requests and zero paid calls, including Ctrl-J editing while idle/running.
Native Linux passes 308 selected tests plus the release build. Source snapshot
`baed9d2007a4546434126d1430381d7f692129056a612ed2c6b3d6bad9c0210d`
and server executable
`119e807ee4815cf57730b0dff7e602d23e97806ec5b9d9e4be3300f2232bede3`
are bound by a completed native build receipt.

The user-approved cloud turn cap is now 32768; the original request-budget
decision, limit and expiration are preserved without refill. An actual HTTPS
read verifies the same persistent identity and active saved memory after the
backed-up, schema-compatible code cutover. A versioned private Windows client
update preserves existing running windows and drafts. No paid inference was
initiated by this repair or acceptance. Local evidence:
`<local-evidence>/lain-chat-fixes-2026-11-05/`
(`final-acceptance.json`, `linux-build-receipt.json`, `native-terminal/report.json`,
`independent-review.txt`). These local artifacts are outside Git; credentials and
personal chat evidence must stay private.
