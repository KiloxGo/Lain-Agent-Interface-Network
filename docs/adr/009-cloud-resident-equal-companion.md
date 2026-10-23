# ADR 009: Cloud-resident continuity, equal companionship and QQ-first delivery

Date: 2026-10-23 (Asia/Shanghai).

Status: accepted product direction; target implementation design, not implemented.

## Context

The user's detailed product discussion identified a mismatch between the desired
daily relationship with Lain and the previous control-plane-first sequence.
They want one cloud-resident Lain, natural shared memory across interfaces,
autonomous reading and private contact within agreed limits, and local PCs that
execute and report work. QQ companionship is the first delivery priority.
The relationship is explicitly equal: no master/servant framing or “主人”.

## Decision

1. Keep one cloud authority for identity, memory, experiences and relationship
   understanding; devices provide execution, observations and bounded caches.
2. Prioritize useful daily QQ interaction before complete multi-worker management.
   The proposed NapCat arrangement separates a read-only user-account observer
   from Lain's own bidirectional account. Only selected social sources are collected.
3. Develop scoped standing authorization for reading, organizing, remembering,
   private notes and private contact. Third-party effects remain separately
   authorized. Models cannot create grants or approve worker permissions.
4. Support correctable autonomous memory with provenance. A temporary impression
   may guide gentle interaction but is not silently promoted to permanent fact.
5. Archive accessible visible worker conversations both inside and outside Lain;
   reading an external session never silently acquires its control authority.
6. Preserve separate conversation, source archive, derived summary, experience,
   memory, work and execution-evidence concepts. Retrieval does not grant disclosure.
7. Preserve worker intelligence in external harnesses, typed provider boundaries,
   secret separation, truthful uncertainty and no blind mutation replay.
8. Use MaiBot as a researched chat reference, not as a predetermined dependency,
   replacement Kernel or proof of runtime correctness.

Normative product details: [PRODUCT](../PRODUCT.md).
Design proposals: [resident](../RESIDENT_DESIGN.md), [chat](../CHAT_DESIGN.md).

## What this supersedes

| Previous decision | Revised scope |
| --- | --- |
| ADR 004: CLI-first is the next product priority | QQ daily companionship is next; CLI remains supported |
| ADR 004: every Actor mutation requires per-action confirmation | Current code keeps this; future scoped standing grants replace the universal target rule |
| ADR 008: all model-selected memory must be individually reviewed | Current records keep their meaning; future authorized autonomous writes retain provenance and correction |
| ROADMAP: World/experience and scheduling must wait behind complete worker phases | Minimal experience and bounded activity are part of the new QQ-first sequence |
| Local Kernel/worker co-location as sufficient deployment model | Target cloud authority plus paired device execution; protocol not yet selected |

ADRs 004 and 008 remain valid historical explanations and descriptions of current
code. ADRs 001–008 are not retroactively rewritten as though new behavior existed.
The previous prompt-enhancement Off default and mode UI remain proposals;
faithful dispatch, original text, exact sent text and transparent differences remain requirements.

## Consequences and migration constraints

This is a documentation change. It grants no new live QQ access, deployment,
paid inference, process control or data migration. Existing runtime approval and
memory gates remain until a separately scoped implementation replaces them with
tested authorization. No fabricated confirmation timestamps for autonomous memory.

Cloud remote security, device pairing/revocation, outbound reconciliation,
retention/deletion, model budgets and cross-device identity need concrete designs.
Do not expose the present loopback API or copy all device configuration as a shortcut.
The current compiled persona still uses historical owner terminology and needs a
separate behavior change with appropriate validation; this ADR changes the target.

Phase 6.6/7.1 real-provider and native Windows/Linux acceptance gaps remain open.
The 2026-10-19 macOS deterministic evidence is historical evidence, not a new test run.
No milestone completion or implemented distributed runtime is claimed.
