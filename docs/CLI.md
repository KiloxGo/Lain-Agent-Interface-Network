# CLI-first local development

Windows PowerShell and macOS zsh are equal development targets, sharing the same
Rust crates, commands and authenticated HTTP/SSE API. Linux remains the eventual
server target. `web/` is retained and frozen; npm, Vite and a browser are optional.

## Install the native binaries

Install stable Rust and a C compiler (MSVC Build Tools on Windows; Xcode Command
Line Tools on macOS). From the checkout, in either shell:

```text
cargo build --locked --workspace
cargo install --locked --path crates/lain-kernel --bin laind --force
cargo install --locked --path crates/lain-client --bin lain --force
lain setup --local
lain up
```

`lain up` stays in the foreground. In another terminal:

```text
lain doctor
lain status
lain profile list
lain chat
```

Cargo installs into its configured bin directory (normally `$HOME/.cargo/bin`
on macOS and `$env:USERPROFILE\.cargo\bin` on Windows). If PATH does not select
Rust's binary, use the explicit path. PowerShell development without installing:

```powershell
.\target\debug\lain.exe setup --local
.\target\debug\lain.exe up
# Another PowerShell:
.\target\debug\lain.exe doctor
.\target\debug\lain.exe chat
```

macOS zsh development without installing:

```zsh
./target/debug/lain setup --local
./target/debug/lain up
# Another zsh:
./target/debug/lain doctor
./target/debug/lain chat
```

Stop foreground `lain up` with Ctrl-C; Windows also supports Ctrl-Break. The
launcher and child share their native console/process group. It waits for laind's
existing graceful HTTP/worker/storage shutdown; it is not a service installer.
A second `up` checks the authenticated daemon's identity/version and readiness,
then returns `already_running`. It never kills an occupying process. Old daemon,
authentication mismatch and unrelated service occupancy fail with diagnostics.
Port-check/start races can still cause a bind failure, safely without killing
anything. No implicit local startup occurs from other CLI commands.

## Local configuration and credential boundary

| Host | Default private directory | Protection |
|---|---|---|
| Windows | `%LOCALAPPDATA%\Lain` | Current user's SID owns the directory/files; protected owner-only DACL with inheritable FullControl on the directory |
| macOS | `$HOME/Library/Application Support/Lain` | Owner-owned directory 0700; ordinary single-link files 0600 |
| Linux | `$XDG_CONFIG_HOME/lain`, otherwise `$HOME/.config/lain` | Same Unix owner/mode checks |

`LAIN_LOCAL_DIR` selects a different absolute directory outside a Git checkout.
Settings: `client.json` (version, local URL, absolute SQLite path and optional
explicit Actor JSON/env-file paths). Authentication:
`kernel-token` (32 bytes from the OS random generator encoded as 64 hexadecimal
characters). They are separate files; neither is source code or SQLite state.
`setup.lock` serializes setup and atomic settings replacements. All are private.
No encryption/keyring is invented.

Save Actor settings with `lain config actor set --file <actor.json>` and a protected
credential source with `lain config actor env-file --file <actor.env>`. Inspect with
`lain config actor`; use `lain config actor clear` to remove selections without
deleting their files. `lain up` reuses saved paths, or accepts `--env-file <path>`
for one launch. `lain doctor --env-file <path>` checks that same selection.
Only explicit trusted files are loaded; no workspace `.env` discovery/shell execution.
Precedence, supported keys, native permission preparation and safe explicit restart
are in [LAIN_ACTOR.md](LAIN_ACTOR.md#save-an-explicit-local-selection).

First setup restricts the directory before writing any credential and creates
files exclusively. Repeated setup validates and reuses the existing token/settings;
it never rotates/overwrites them. Interrupted setup can reuse an already-created
valid token. A malformed existing file, missing token in an existing setup, wrong
owner, permissive mode/ACL or linked file fails closed with files retained. Setup
does not silently repair permissive existing credentials. Restore their correct
permissions deliberately or restore a trusted backup. Tokens are absent from
normal output, command arguments, Git, model input, logs and operational history.

Windows ACL handling runs native Windows PowerShell's .NET security APIs with a
literal data path and a cleared/OS-allowlisted environment; no shell source interpolation or Lain credential in that process.
ACL inspection checks the current SID and rejects non-owner access. PowerShell
must be available; unsupported filesystems/ACLs fail explicitly. Elevated sessions
whose files are owned by a different SID may need an intentional owner correction.
Unix uses native filesystem modes and owner/link checks. Tests run only on their
real OS; neither platform is simulated on the other.

Existing `LAIN_API_TOKEN` overrides the saved token. With an explicit token,
`LAIN_URL` selects a remote endpoint; without it, an endpoint different from saved
local configuration is refused to avoid sending the local credential elsewhere.
With a token but no URL, the legacy `127.0.0.1:8787` default remains. `lain up`
rejects `LAIN_URL`, `LAIN_LISTEN` and `LAIN_DATABASE_PATH` overrides: it uses the
explicit local setup. Use `laind` directly for custom deployment configuration.
`LAIN_DAEMON_PATH` selects an explicit native binary; otherwise `laind` must sit
beside the running `lain`. The launcher's local database survives exits/restarts.
Existing project `data/lain.sqlite3` is not automatically imported; to retain
previous sessions, deliberately point local client.json's database to that trusted
absolute database while both daemons are stopped. Preserve WAL/SHM backup rules.

Actor configuration and `LAIN_SECRET_*` are daemon settings, inherited at launch;
other terminals need only saved local authentication. See [LAIN_ACTOR.md](LAIN_ACTOR.md).

## Diagnostics and PATH conflicts

`lain doctor` performs no inference/provider call. It reports CLI/native executable,
authenticated Kernel reachability/readiness, SQLite quick_check, backend executable,
exact supported Codex version, profile health/missing SecretRefs and Actor model
configuration. Local prerequisite health is not a claim about account/model quota.
`--json doctor` exposes the structured report. Unreachable/authentication/storage
failure exits nonzero. Backend/Actor/profile problems are reported separately so
worker commands can remain usable without Actor credentials.

An obsolete npm `lain` can shadow Rust on Windows. Inspect:

```powershell
Get-Command lain -All
where.exe lain
& (Join-Path $env:USERPROFILE '.cargo\bin\lain.exe') doctor
```

zsh: `whence -a lain`. Doctor lists matching PATH files and its own executable.
Check the installed Rust `lain --version` and use an explicit path until you
choose how to resolve the conflict. Lain never uninstalls npm packages or rewrites
PATH automatically. It cannot diagnose its own shadowing if the obsolete command
runs first; the explicit Rust invocation is the fallback.

## Commands

Existing work/profile/approval commands and the worker TUI remain available.
Worker workspace arguments are verbatim **daemon-host paths**, even in local
mode; there is no implicit current-directory or `--here` resolution.

```text
lain work list
lain work start --workspace <absolute-daemon-path> --profile codex/local
lain work send <session-id> "inspect tests"
lain work watch <session-id>
lain work history <session-id>
lain profile list
lain approval list <session-id>
lain approval approve <session-id> <approval-id>
```

Watch only observes HTTP/SSE; Ctrl-C never cancels work. Existing attach/TUI
behavior is unchanged. UUIDs remain exact; shortened/current work references are
not implemented in this phase.

```text
lain chat
lain chat --conversation <conversation-id>
lain chat --timing
lain ask --timing "你好"
lain --json ask "What work is running?"
lain --json ask "Continue" --conversation <conversation-id>
lain --json ask "Same text" --conversation <id> --request-id <same-request-uuid>
lain conversations
lain conversation <conversation-id> --after <cursor>
lain actor show <turn-id>
lain actor approve <turn-id> <action-id>
lain actor deny <turn-id> <action-id>
lain actor cancel <turn-id>
```

REPL: `/help /status /work /conversations /history /new /exit`. Confirmation shows
the exact persisted tool/arguments and defaults to denial; EOF retains a pending
proposal. Explicit `actor approve` is an owner decision, never automatic approval.
Noninteractive ask returns `pending_confirmation` with proposed actions and does
not execute them. JSON turn snapshots contain conversation/turn/request IDs,
messages, typed actions/results, errors, usage and final state. Errors are a JSON
envelope on stderr with nonzero exit; stdout contains the command's structured
result when one exists. Unknown/failed/cancelled ask or explicit action outcomes
exit nonzero. Watch is JSON-lines when requested.

On stream loss the client resnapshots the **same turn**, at most three attempts,
and deduplicates message IDs/revisions. It never resubmits text automatically.
If a submission response is lost, use the reported conversation/request ID and
identical text to retrieve the original accepted request. A changed payload under
that request ID conflicts. One unfinished turn per conversation is allowed.
Ctrl-C during generation explicitly asks the Kernel to interrupt that Actor turn;
possibly delivered mutations remain uncertain. Ctrl-C at the input prompt exits
chat; workers keep running. On resume the recent 20 turns include the latest
pending/running state; older records use conversation pagination.

Assistant previews are visible before provider completion and remain ephemeral
until a validated complete message commits. Final snapshots do not repeat already
displayed text. Failure/discard notices identify partial previews as unconfirmed.
Terminal controls are escaped, including Unicode bidi controls. Input is owned by
one scoped native terminal read; no separate stdin reader runs during generation.
Empty Enter at `You>` keeps the current prompt. LF and CRLF are handled explicitly;
the paired LF cannot answer a subsequent confirmation. Empty Enter at `[y/N]`
denies. Ctrl-C/EOF at confirmation exits and retains the proposal for review.

`--timing` emits safe measured durations to stderr, including in JSON mode:
CLI submission/first-visible/total milliseconds plus Kernel queue and per-request
first-body-byte, useful-text, preview, completion and commit milliseconds. Null
means no observation. CLI totals cover the active phase and exclude owner waiting;
provider timings are relative to that provider request. No conversation content
or credentials are included. This measures observed latency rather than guessing
why a provider is slow.

## Native acceptance

```powershell
powershell -NoProfile -File scripts/check-native.ps1
```

```zsh
zsh scripts/check-native.zsh
```

Both run native build, fmt, all-target Clippy, deterministic tests and CLI smoke.
The smoke refuses a mismatching `--platform`; it uses native processes, permissions
and shutdown signals, a local deterministic Responses server, and the existing
fake Codex App Server. macOS additionally exercises actual controlling PTYs:
LF/CRLF, empty/slash/consecutive inputs, gated Unicode streaming before completion,
no-preview/failure, JSON/timing separation, denial, approval, Ctrl-C during input
and generation, EOF, restoration, reconnect checkpoints and detached inference.
Windows ConPTY input acceptance remains pending. Linux can run the same Cargo checks and
`python3 scripts/native_smoke.py --platform linux` on a native Linux host.
These are manual entrypoints, with no paid-model or oversized CI matrix.

Acceptance status for this change is recorded in DEVLOG.md. Windows and Linux
are unverified until these checks actually run on those hosts. Real provider
acceptance is separately opt-in, never inferred from fixture success.
