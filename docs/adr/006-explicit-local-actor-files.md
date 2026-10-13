# ADR 006: Explicit local Actor configuration and credential-source files

Date: 2026-10-13. Status: implemented; native macOS verified, Windows/Linux pending.

## Decision

The local launcher records explicit canonical Actor JSON and env-file selections
in its existing private per-user settings. Commands are `config actor set --file`,
`config actor env-file --file`, `config actor [show]` and `config actor clear`.
Settings replacement is atomic under the existing setup lock; selected files are
never rewritten or migrated. The owner's ignored checkout Actor JSON is preserved.

`up --env-file` selects a file for one launch; `doctor --env-file` compares the
same desired selection. Saved files load automatically on later `up` launches.
Precedence is process environment > selected env file > saved non-secret path >
defaults. Relative env-file paths anchor to that selected file's parent. No cwd or
worker-workspace discovery occurs. Direct `laind` still reads its environment.

The deliberately limited file format is literal unquoted UTF-8 KEY=VALUE with
LF/CRLF and whole-line comments. Only Actor/persona paths and namespaced
LAIN_SECRET variables are accepted; there is no shell evaluation, interpolation,
arbitrary environment injection, API-token override or executable selection.
Credentials remain outside Git in ordinary owner-only files/private directories:
0600/0700 on Unix, protected current-owner ACLs on Windows. Unsafe sources fail
closed. Values are zeroizing transient launcher buffers and child environment;
views expose paths, variable names, availability and non-secret revisions only.

The authenticated daemon exposes startup revisions of validated non-secret Actor
JSON bytes and persona content. Doctor can report changes, including conservative
formatting-only changes, without hashing or comparing credentials. Older daemons
may be unobservable; remote mode does not compare local files. Provider access
remains unprobed. Credential rotation requires an explicit next start.

No hot reload or process adoption is added. The owner inspects active work, stops
their owned foreground launcher gracefully, then starts it with the same database.
An existing mismatched daemon is reported with open worker count and is never
killed/replaced by `up`. Unknown services and remote daemons are not controlled.
Restart preserves conversations/IDs and follows existing worker recovery rules,
including uncertainty and no mutation replay.

Native fixture checks cover export-free startup, spaces in paths, LF/CRLF env
parsing, permissions/links, precedence, mismatch/no interruption, explicit files,
persona changes, restarts and preserved Actor/worker IDs without inference on
configuration checks. Native Windows ACL tests are present but remain unverified.
