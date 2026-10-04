# Development Policy

**Status:** normative foundation  
**Last reviewed:** 2026-10-04

## 1. Purpose

This document captures repository-level engineering habits that should remain stable while the new implementation is built.

## 2. Local-first development

The primary development loop is a local coding-agent session plus local tools.

Before finishing a change, run the smallest useful deterministic validation locally.

Typical Rust work should prefer:

- `cargo fmt --check`
- targeted `cargo test`
- `cargo clippy` when the changed code benefits from it

Do not run the entire world when a focused test gives the same confidence.

## 3. GitHub Actions

GitHub Actions are optional infrastructure, not the center of the development workflow.

If CI is added later, keep it cheap and deterministic.

Good candidates:

- formatting checks;
- targeted compile/test jobs;
- one primary Linux toolchain;
- lightweight frontend typecheck/lint when needed.

Avoid by default:

- agent/LLM review on every push;
- model calls in CI;
- token-consuming evaluation workflows;
- broad OS/version matrices;
- repeated full integration environments;
- high-frequency scheduled workflows;
- workflows whose runtime cost is larger than their practical signal.

Expensive checks, benchmarks, or agent evaluations should be manually triggered or run for releases when there is a concrete reason.

The goal is fast feedback, not CI theater.

## 4. Milestone commits

The project has an established milestone style: short phase label plus a quoted phrase.

Examples from project history:

- `first echo: "Cogito, Ergo Sum"`
- `birth eve: "I am that I am"`
- `connection: "I and Thou"`
- `woven: "The Thread of Memory"`

Use this style when a commit marks a genuine milestone: a new layer becoming real, a major architecture boundary landing, or a meaningful product capability becoming usable.

Small implementation commits should remain straightforward and descriptive.

Do not force poetic commit messages onto routine fixes.

## 5. Milestone quality bar

A milestone commit should ideally leave the repository in a coherent state:

- relevant docs match the implementation;
- targeted tests pass;
- no known secret is committed;
- temporary scaffolding is either removed or clearly marked;
- the milestone can be explained in one sentence.

## 6. Legacy code

The `legacy/v0` branch preserves the previous implementation.

Do not copy large legacy modules into v1 merely because they already exist.

Legacy code may be consulted for:

- experiments that produced useful lessons;
- old UI/assets;
- Live2D/audio work;
- prior prompt/persona ideas;
- historical behavior.

Any reused code must fit the current architecture rather than pulling the architecture back toward v0.
