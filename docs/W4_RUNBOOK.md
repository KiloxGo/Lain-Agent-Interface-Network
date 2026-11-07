# W4 local operating and reproduction guide

2026-11-03. This is a local development handoff, not an installed personal instance
or real-provider/cloud acceptance. [W4 acceptance](W4_ACCEPTANCE.md) separates those
gates. Keep all credentials, device settings, runtime state and temporary evidence
outside the source checkout. Existing personal state is not a test fixture.

## Native Windows deterministic reproduction

The tested machine has a separate native GNU Rust/Python environment at
`%LOCALAPPDATA%/Lain-Test`. In PowerShell from the repository:

```powershell
& "$env:LOCALAPPDATA/Lain-Test/Dev-Environment.ps1"
$w4BuildTarget = "$env:TEMP/lain-q1-tools/target-w4-local-check"
cargo fmt --all -- --check
cargo test --workspace --target-dir $w4BuildTarget --locked --offline
cargo clippy --workspace --all-targets --target-dir $w4BuildTarget --locked --offline -- -D warnings
cargo build --workspace --target-dir $w4BuildTarget --locked --offline

$env:PYTHONDONTWRITEBYTECODE = '1'
$w4Python = "$env:LOCALAPPDATA/Lain-Test/python/Scripts/python.exe"
$w4Runner = "$env:LOCALAPPDATA/Lain-Test/native-test.py"
$w4Bins = "$w4BuildTarget/debug"
& $w4Python $w4Runner scripts/device_host_smoke.py --bin-dir $w4Bins
& $w4Python $w4Runner scripts/w4_work_smoke.py --bin-dir $w4Bins --artifacts <absolute-output-directory> --portrait-file <absolute-selected-A-text-file>
& $w4Python $w4Runner scripts/desk_terminal_smoke.py --bin-dir $w4Bins --artifacts <another-absolute-output-directory> --portrait-file <absolute-selected-A-text-file>
```

Replace angle-bracket values with explicit paths, quoting paths with spaces.
The dedicated check target keeps compilation separate from an already open
preview. Close processes using that same check target before rebuilding it, or
deliberately select another target. The latest recorded prebuilt preview set is
the external session directory's `lain-w4f-2026-11-04/native-bin`; the manual host
selects it explicitly. A source rebuild does not automatically replace that set.
This machine's selected art is the session artifact directory's
`lain-art-research-2026-11-02/candidate-a.txt`. It is not redistributed in Git.
The external test wrapper adapts ownership of **new synthetic fixtures only** in
an elevated environment; it never repairs or takes ownership of existing private
user paths. On a normal native development environment run the scripts directly
with its native Python and built binaries; no emulation counts as native evidence.
Each process script must exit successfully and confirm cleanup. Fixture work uses
no paid model or real QQ; a rendered VT image is explicitly not a desktop capture.

## Explicit operator setup and start

1. Have the intended Kernel running with its separately selected Actor/provider
   configuration. Development does not supply live keys or enable paid inference.
   Configure the administrator client using [CLI setup](CLI.md); do not paste keys
   into task text, command arguments, Git or output.
2. Enroll, inspect and claim the intended device/workspace using
   [device pairing](DEVICE_PAIRING_ACCEPTANCE.md). Registration is not online status.
3. On that device, explicitly configure and check its protected execution policy:

   ```text
   lain device host-configure --directory <absolute-private-device-dir> --codex-path <absolute-native-codex-executable>
   lain-device-agent --check --directory <absolute-private-device-dir>
   lain-device-agent --directory <absolute-private-device-dir>
   ```

   The default is read-only. Broader workspace authority must be deliberately
   configured and remains distinct from individual Worker permission decisions.
   The check is read-only; readiness does not prove a running service/provider.
   The final command is an explicitly owned foreground process, not an installer.
4. Open the Rust client, retaining the displayed conversation ID:

   ```text
   lain desk --portrait <absolute-local-portrait-file>
   lain desk --conversation <existing-conversation-id> --portrait <absolute-local-portrait-file>
   ```

   All client surfaces use the same Kernel. Closing the frontend leaves work
   running; reopening with the ID restores persisted observations, not unsent
   process-local drafts. No npm/browser dependency is needed.

## Everyday task interaction

- Ask Lain to delegate an explicit task. Review the actual device/workspace/
  profile/sandbox, original records, outbound text and Changed/Unchanged status.
  Read the full scope before explicit confirmation; a queue receipt is not work
  completion. The owner JSON CLI path is also in [DEVICE_WORK.md](DEVICE_WORK.md).
- Tab opens Work/Devices; Enter inspects the selected task. Continue chatting with
  Lain while the device owns its Worker. Work and Lain drafts are separate.
- In task details, F4 reviews a current Worker permission. Only actual offered
  choices are available. Cloud recording, reported device receipt and reported
  Worker application are separate; missing confirmation remains unconfirmed.
- F6 opens input to the same known idle completed/failed Worker session. Enter
  previews; after reading, Ctrl-Y sends. Running steering is unsupported by this
  path. A changed, cancelled or Unknown target cannot become replacement work.
- F8 reviews the exact task cancellation target. Ctrl-K after full reading requests
  a stop. Esc closes without effect; returning to Lain keeps its draft. A recorded
  stop request is not observed termination and does not undo earlier changes.
- Ctrl-R reads current facts. `/decision-retry`, `/followup-retry` and
  `/cancel-retry` explicitly retain their pending original request/scope. They do
  not replay native uncertain effects or silently use newer permissions/revisions.
- Missing chat-model/credential configuration is shown explicitly; the input is
  retained without a new submission. Correct the intended Kernel configuration
  using the existing Actor setup, refresh with Ctrl-R, then explicitly send. This
  metadata check does not validate paid-provider access or automatically send a draft.
- Ctrl-Q closes the frontend. Explicit device-host shutdown drains owned runtimes;
  wait for the host to exit before replacing its policy. Do not delete journals
  or infer presence from a retained lock file.

## Recovery and limits

| Observation | Correct next action |
| --- | --- |
| Disconnected/old snapshot | Read/reconnect to the same IDs; cached state is historical |
| Unknown submission or permission application | Inspect original binding, reports and protected journal; never requeue or replay merely to obtain a cleaner status |
| Changed review/revision | Reread and explicitly review the new target |
| Cancellation requested | Wait for separate device evidence; inspect a completed/failed result if it won the race |
| Expired/revoked/missing workspace or unsafe local ACL | Correct the intended configuration deliberately; no automatic scope repair |
| No native full terminal entry | Backend capability is absent; use supported structured interaction |

Real paid Codex, cross-device TLS, Linux/macOS and multi-day operation are separate
acceptance. The user confirmed the actual Windows Terminal wide visual on
2026-11-04; narrow resizing/editing retains separate native ConPTY evidence.
The prepared **local-only** manual host and instructions are under the session
artifact directory's `lain-w4e-2026-11-03/manual-review.md`; its `--check-only`
rehearsal passed with no model/QQ activity and cleaned temporary state. It opens no
terminal window automatically. Human GUI review is not inferred from that check.
The manual host prints a Python executable/`manual-client.py` command for the
human to run in their existing terminal. It does not require a PowerShell script
or execution-policy change; older `manual-client.ps1` instructions are superseded.
The host's headless check executes the actual generated Python launcher's
read-only connection path and verifies its isolated empty workspace list.

## Conditions for the next real cross-machine acceptance

This is preparation for a separately authorized run, not authorization to deploy,
access a server, spend provider credit or change a personal instance. The current
same-machine native reports do not satisfy these conditions.

| Required input or preparation | Concrete acceptance condition |
| --- | --- |
| Two actual selected machines | Identify the Kernel host and the native PC, OS/architecture, operator and intended endpoint. Build/test the Kernel natively on its host; current Windows evidence does not validate Linux/macOS. Record binary hashes and compatible protocol/schema versions. |
| Explicit deployment scope | Select a disposable server runtime/database and private PC device directory, or separately approve a real-instance upgrade with a stopped full backup and rollback plan. Do not copy the personal PC database to manufacture cloud identity. |
| Reachable authenticated HTTPS | Provide the actual hostname, certificate/trust chain and intended network access. Non-loopback device HTTP is rejected by the implementation. Retain certificate verification, no redirects and separate owner/device credentials; never put tokens in URLs or test output. |
| Real streaming route | Verify authenticated SSE/change streams through the actual TLS route, including any proxy buffering/idle limits, disconnect detection and reconstruction from persisted snapshots. A local fault proxy is not this evidence. |
| Selected native execution scope | Record the PC executable/version/capability probe, device/workspace IDs, canonical dedicated test workspace, profile, sandbox, finite deadline and local offline grace. `--check` must pass prerequisites; registration/readiness still does not prove presence or a Worker start. |
| Protected independent credentials | Provision owner, device and provider access only to their intended runtime. Verify native owner-only files and that the cloud does not receive PC provider credentials. Do not import or modify global Worker configuration as part of enrollment. |
| Separate model authorization | For a real Actor and real Codex, explicitly select providers/models, maximum calls or spending, deadline and stop conditions. Lain's own inference ceiling is not a Codex billing ceiling; Worker/provider limits need their own agreed control. Closed earlier campaigns supply no remaining allowance. |
| Bounded test data and effects | Use synthetic task content and a dedicated workspace, initially read-only. Any file-write test needs an explicit selected path and permission. Leave QQ/Telegram, private archives, collection and proactive contact disabled unless separately selected. |
| Evidence and cleanup owner | Select an external artifact directory on each machine and align their clocks enough to interpret deadlines. Capture UTC timestamps, normalized IDs, hashes, reported state and actual process observations. Keep tokens/private content out of artifacts; stop only the explicitly owned test processes and record cleanup. |

Run transport and identity acceptance with the structured fixture before enabling
paid providers. After the separate provider/budget decision, repeat the relevant
scenarios with the actual selected Codex runtime; do not replace one evidence
category with the other.

The cross-machine scenario record should include:

1. Pair/claim the selected PC over the actual HTTPS route; wrong/device credentials
   cannot use owner decisions or inspect another device. Prerequisite reads do not
   start a service or a Worker.
2. Dispatch one exact reviewed read-only task, retaining conversation/command/
   execution/session/turn identities. Chat and Work navigation remain responsive.
3. Review actual supported allow/deny requests. Capture cloud recording, device
   receipt and native Worker confirmation independently; an absent confirmation
   stays unconfirmed even if a turn ends.
4. Close/reopen the client and restart the selected Kernel/device host separately.
   Observe the original native binding and continue its known idle session with a
   distinct new turn; count actual backend starts/submissions to detect duplication.
5. Exercise receipt loss and a real network interruption across the two machines.
   Recovery reads original facts; uncertain mutations are not replayed or replaced.
6. Review/cancel a running task and record requested versus observed termination,
   including a completion race. Preserve failed commands, incomplete history and
   unverified artifact-reference labels in the returned result.
7. Inspect the actual Windows Terminal/font and selected viewport sizes, then
   deliberately stop the test and verify no owned background runtime is left.

For each case record expected observation, actual observation, evidence path,
unverified facts and pass/fail. A successful HTTP response, a process starting or
a Worker reply alone does not establish task or whole-stage completion.
