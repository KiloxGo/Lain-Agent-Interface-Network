# Real Work web-task acceptance — 2026-11-05

## Result and scope

The selected native Windows client, existing cloud Kernel and installed native
Codex complete three real turns in one session for an isolated static webpage.
The test uses actual provider execution and Chrome, not fixture model/backend
responses. This establishes the selected happy path, including a frontend restart
during execution; it is not universal reliability or other-platform acceptance.

The product's frozen `web/` frontend is unchanged. The generated test website is
outside the repository. The original device's read-only policy, foreground host,
global Codex settings, personal conversation, QQ bindings and cloud release stay
unchanged. No Git commit, push or public website deployment occurs.

## Environment and authorization

The user requests a real small-webpage end-to-end test with no blocked steps.
The existing selected Work/Codex design supplies the procedure and authentication
background; no new authentication bypass or setup interview is needed. The test
adds a separate two-hour device registration, one isolated writable workspace,
protected device credential/policy and a dedicated foreground host using the
installed checked Codex binary. It creates one separate cloud conversation.
The original execution host continues throughout.

The owner credential is read from the existing protected installation, only into
process memory/environment. It is not copied into the website, prompts, report
or normal logs. Worker processes receive no cloud owner/model credential env.
The webpage uses native HTML/CSS/JavaScript, no dependency installation or remote
resources. Its local preview binds to loopback only.

## Cases and observed evidence

| Case | Layer | Observed result |
| --- | --- | --- |
| Native prerequisites | CLI/cloud | Canonical workspace, installed supported Codex and active device authority pass `--check`. |
| Exact creation | Real TUI/API | Work → N selects only the test project; WorkspaceWrite, original task and deadline are reviewed before Ctrl-Y. Exactly one command is recorded. |
| Real execution | Native Codex/files | First turn produces `index.html`, `styles.css`, `app.js`, `README.md`, with a final Succeeded/Completed receipt. |
| Parallel conversation | Real TUI/Actor | During native execution, Chat asks for status; two model requests and a read-only device-work lookup complete without interfering with the task. |
| Real page delivery | HTTP/Chrome | HTML, CSS and JavaScript return 200 and render the named page. |
| Initial interactions | Chrome | Add by button, toggle completed, counter updates, whitespace rejection and reload persistence pass. |
| Same-session follow-up | Real TUI/API/Codex | F6 adds All/Incomplete/Completed filters and Clear completed. Parent link, inherited workspace/sandbox and identical native session are verified. |
| Close/reopen while working | Real TUI/API | Client closes while turn two is Running. Host continues; reopening restores Work without submitting a duplicate command. |
| Follow-up interactions | Chrome | Filters change only visible tasks; stored total remains two. Clear completed removes only the completed task; the incomplete task survives reload. |
| Natural-language delegation | Real Actor/TUI/Codex | Chat identifies the current child and proposes `device.send`. Review shows original record, Changed payload, difference, device/parent revisions and exact text. Explicit owner confirmation creates turn three in the same native session. |
| Environment check | Native process | Independent execution of the existing bundled Node with `--check app.js` exits 0, with empty stdout/stderr. |
| Browser fix | Fresh Chrome context | Final document contains an inline SVG favicon. Network contains only the three successful local document/CSS/JS requests; console has no errors/warnings. |
| Keyboard/mobile/desktop | Chrome | Enter adds tasks; accessible input label exists; actual 390 px mobile and 1440 px desktop viewports have no horizontal overflow. |
| Complete history | Real TUI | Scrolling the restored timeline exposes all three original/follow-up instructions and their respective results, including the first result. |
| Cleanup | Cloud/native process | Test host stops normally; test registration is revoked; its dedicated credential is removed. Original host remains running. Audit receipts, generated website and local preview are retained. |

Three commands, three Succeeded receipts, one native Codex session. Recorded
creation-to-final-cloud-receipt durations are 121.0 s, 102.5 s and 56.3 s. These
include model execution and reporting; they are observations for different tasks,
not a controlled performance comparison. Two Actor turns use four model requests
in total. No Worker permission request occurred, so real interactive Worker
permission resolution is not established by this test.

## Discovered issues and resolution

1. The initial test-directory preparation ran as the restricted tool user while
   the installed native host runs as the actual Windows user. Canonicalization
   failed before pairing or dispatch. The empty test directory was verified and
   recreated by the actual execution user with its private ACL; no existing
   project ACL was changed.
2. Node is absent from the host PATH. The first turn reports one failed command
   and uses a V8 syntax parser instead. The third turn is given the existing
   bundled Node executable's explicit path; it runs the real syntax check. The
   tester independently repeats that exact check with exit 0. Global PATH stays
   unchanged. Automatic runtime discovery remains a usability improvement.
3. Initial browser navigation requests `/favicon.ico` and gets 404. This is sent
   through Lain's actual proposal/review/follow-up path. Codex adds an inline SVG;
   a fresh isolated browser context confirms no favicon network request or
   console error. The tester does not edit the website implementation.
4. Two test-driver waits used the wrong UI text or assumed the first round was
   initially visible while the timeline correctly followed the latest result.
   Their captured screens show normal product state, not a stuck task. Scrolling
   to the beginning/middle/end independently verifies the real three-round view.

Project enrollment and writable local policy still need explicit preparation.
This test does not claim the daily device's read-only policy was upgraded, that
arbitrary projects can be edited without setup, or that runtime discovery is
already automatic. The Kernel's `artifact_verification: not_performed` and
`goal_acceptance: not_established` remain unchanged: the independent browser/file
acceptance is recorded here, not fabricated as a device assertion.

## Evidence and repeat procedure

Private local artifacts are in the session visualization directory's
`lain-web-e2e-2026-11-05/`. Key files:

- `driver.py`, `common.py`, `actions.jsonl`: exact test tasks and UI actions;
- `final-commands.json`, `final-evidence.json`: command lineage, durations and Actor facts;
- `independent-file-check.json`: actual final file hashes and Node exit status;
- `before-ui-restart.json`, `08-work-after-restart.png`: Running state and restored UI;
- `12-natural-review-1.png`, `13-natural-review-2.png`: complete explicit owner review;
- `15-three-round-history-top.png`, `17-three-round-history-middle.png`,
  `18-three-round-history-end.png`: actual ConPTY VT renders, not desktop screenshots;
- `web-final-desktop.png`: original Chrome screenshot, unmodified;
- `cleanup.json`: revocation, credential removal and retained preview facts.

The terminal image helper's fixed heading says "synthetic fixtures" because the
task data is synthetic. These captures in this directory use real cloud/provider
execution, unlike the repository's deterministic fixture acceptance scripts.

For read-only reinspection, the external `common.py` uses the existing private
installation. From that artifact directory, its `state()`/`cleanup-target.json`
and `api('GET', '/v1/conversations/<recorded-conversation>/device-work')` recover
the exact three receipts without replay. Compare their workspace, continuation
parents, native session and state with `final-commands.json`.

Open the retained `runtime/web-project/index.html` or serve that directory on
loopback, then repeat the browser cases above in a fresh browser context. Use the
bundled native Node `--check <absolute-project-path>/app.js`. Compare file SHA-256
values to `independent-file-check.json`. The recorded URL is local/temporary and
is not a published demo.

For a new live run, create a new private empty workspace, separate conversation
and finite device registration. Never blindly rerun the original driver's POSTs:
its setup and immutable dispatch receipts belong to this finished run. Preserve
the new exact request/receipt identities, review every proposed effect, then stop
and revoke only that newly created test device after acceptance.

Native Linux/macOS Work UI, forced network partition, cloud restart, cancellation,
real Worker permission prompts, simultaneous independent tasks and long-term
reliability are outside this selected real-webpage run.
