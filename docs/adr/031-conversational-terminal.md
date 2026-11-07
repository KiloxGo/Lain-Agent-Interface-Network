# ADR 031 — Conversational terminal shell over Kernel-owned work

Date: 2026-11-02. Status: accepted for W4a.

## Context

The user already selected a Claude Code-like terminal with Lain ASCII presence,
Chat/Work modes, a unified dashboard and independent Worker progress. Existing
line chat and Worker TUI implement useful contracts but are separate entrances.
W3b has a separately configured native host; terminal closure must not own it.

## Decision

Use a Rust/ratatui conversational shell as the default `lain` / `lain desk` entry.
Keep legacy `lain chat`, explicit JSON commands and dedicated `work dashboard` /
`work attach`. Reuse the authoritative Actor renderer and Worker detail TUI.
There is one input owner at a time; transitions explicitly release terminal state.

Chat drafts remain editable during generation and survive in-process navigation.
Submission uses immutable original text and stable request identity. Recovery is
read-only; uncertain mutations are not automatically replayed. Lain action review
binds the current exact payload/revision, requires full visible scope and has no
default approval. Worker permissions retain their existing separate contracts.

Device lists, command summaries and details are observational in W4a. Add a typed
bounded owner-only metadata projection over existing commands, without schema or
executor changes. No device credential can call owner APIs. Remote mutation and
service lifecycle interaction remain separately scoped work.

ASCII expressions reflect actual UI/Actor connection states; they do not invent
experiences or introduce a local persona. See [full design](../TERMINAL_DESIGN.md).

## Consequences

Terminal development stays independent of npm. Workers remain Kernel-owned and
continue when the frontend closes. Existing Worker details initially retain their
own layout and shortcuts. One conversation has one active Actor request; users
can prepare the next draft and inspect independent work, without an implicit queue.
Native full Worker terminals require real supported capabilities and are not
simulated by this UI. Cross-device dispatch, device-service management and broader
W4 acceptance are not claimed by W4a.
