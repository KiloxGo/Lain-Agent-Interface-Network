# Private visual conversation acceptance

2026-10-28, native Windows/MSVC. Scope: [ADR 027](adr/027-private-visual-conversation.md).
Implementation is present. The user separately authorized the personal upgrade,
which is now installed and Online; actual provider/QQ visual acceptance remains
pending. The deterministic evidence alone did not authorize the migration.

## Authorized personal installation — 2026-10-28

The owner explicitly approved schema16→24 and current picture/sticker input only
for the existing bot<bot-qq> ↔ owner<user-qq> private binding. No autonomous
real-provider replay or new memory/history/contact/Worker scope was authorized.
The owned hidden console stopped gracefully with no active Worker, inference or
queued delivery. All 43 files were backed up with native DACL copying and exact
file hashes under the protected instance backup directory, including credentials
without displaying their values.

A private backup copy booted schema24 with model and QQ configuration absent.
Eleven existing core tables matched the stopped original exactly, including
identity, messages, memory, permissions, inference decision and delivery receipts.
The preserved backup did not change. The accepted 2d98c70 binary pair was then
installed as releases/visual-chat-2d98c70; the original launcher was updated to its
exact path/hashes. Only receive_images and supports_images changed in selected
configuration. Credential files, kernel token, client settings, provider and the
existing selected-private unlimited decision were retained.

Actual Kernel Ready, QQ Online, schema24 integrity, matching conversation/binding,
available existing SecretRef, disabled memory/history grants and empty device
registry were verified. The new hidden launcher is outside the Codex tool job;
this does not install reboot autostart. No real-model probe or forged QQ input
was sent. At this record's update, no actual picture had arrived on the new path,
so the visual goal is still open for real image/reply evidence.

Supplemental synthetic APNG (including a non-animation poster), animated WebP and
70-frame GIF tests now verify actual sampled colors, frame times and the explicit
64-frame prefix limit. They do not change production behavior or the installed
accepted binary pair. The fixture generator uses Pillow only when regenerating
test assets, not during Rust builds/tests or runtime.
The supplemental final workspace run passes 292 Rust tests (3 manual ignored),
with format and strict Clippy passing. Production decoder code and the accepted
installed binary hashes are unchanged by these test additions.

## Implemented path

The verified friend-private NapCat event commits text and ordered media references
before preparation. Current event CDN URLs or bounded get_image bytes enter a
decoder with explicit resource limits. Standard QQ faces use a dated factual
ID/name mapping. PNG/JPEG/GIF/WebP (including animated WebP/APNG) become bounded
JPEG frames. Market/picture stickers are actual visual input, not summary labels.
The existing private Actor sends those frames through ordinary Responses
input_image parts alongside original text and attributed availability/sample
metadata. No extra caption request, inference budget or Worker capability exists.

Both receive_images (NapCat) and supports_images (Actor) default false. Missing
capability, download/decode failure, flash media, capacity and interrupted reads
produce explicit evidence. Unavailable current pictures cannot be silently ended
with chat_finish. Viewed social stickers may still close naturally; pictures do
not force captions. Model behavior beyond these deterministic gates still needs
real visual-quality acceptance.

## Lifetime and disclosure

- Source bytes: 8 MiB; static dimensions at most 8192 and 24 million pixels.
- Animation: at most 64 decoded frames/64 million pixels, each canvas at most
  four million pixels; first, frame eight and last decoded frame are sampled.
  Reaching the frame limit is conservatively marked truncated.
- Normalized frame: at most 384 KiB, edge at most 2048, no upscaling; white alpha
  compositing and EXIF orientation precede model input.
- Preparation: eight permits across reconnects, durable 20-second deadline;
  read/decode limits remain effective even if a blocked decoding task is cancelled.
- Retained normalized frames: 64 MiB across the Kernel; original ten-minute reply
  expiry is never extended on follow-up or restart. No refetch after interruption.
- A request supplies at most eight recent pictures/24 frames, with bounded
  metadata for the latest 32 media records. Older/unlisted pixels are not supplied.
- URLs, local paths, raw events and base64 are not Input/Actor JSON/event history.
  SQLite frame BLOBs remain private operational data, separate from semantic memory.
  Deletion is logical; SQLite/WAL remnants and separately made backups do not
  provide a secure-erasure guarantee.
- Revocation/withdrawal removes frames. Version/expiry gates cover adapter entry,
  continuation, response, commit, delivery and history. Stale assistant speech and
  tool arguments/results cannot revive expired or withdrawn visual observations.
- Automatic memory promotion is refused after actual pixel influence (including
  retained assistant history). Failed/status-only pictures are not claimed as
  viewed. An exact quoted forget retains independent grant/source checks.

## Deterministic evidence

Rust tests exercise actual decoded pixels/alpha/hash/animation positions,
malformed/oversized data, CDN allowlisting/public addresses, pinned HTTP status/
redirect/encoding/body bounds, parser identity/order/duplicates, explicit
unavailability, atomic receipt/frame commits, preparation timeout, revoke/withdraw,
restart, expiry and stale tool-body/history suppression.

Native process rehearsal:

```powershell
python scripts/qq_visual_smoke.py --bin-dir <native-built-binaries>
```

This uses generated PNG/GIF fixtures, actual authenticated OneBot WebSocket and
Responses HTTP/SSE protocols, generated synthetic credentials and temporary
databases. Optional get_image.base64 is exercised alongside an unusable local
path without opening it. Real CDN/QQ/provider behavior is not inferred from this
fixture.

Final-revision evidence (all zero paid calls/real QQ sends):

| Gate | Result |
| --- | --- |
| Native workspace Rust | 290 pass, 0 fail, 3 manual ignored |
| Format / strict Clippy / locked build | pass |
| Existing Python script unit checks | 14 run: 9 pass, 5 platform skips |
| Native visual protocol path | 9 checks, 12 fake model requests, 8 media reads |
| Existing full chat lifecycle | 36 checks, 83 fake requests |
| Source-derived memory | 5 checks, 16 fake requests |
| Knowledge bridge/sync | 6 checks, 10 GETs/jobs, no model |
| Device identity/report | 5 checks, 3 reports, no model |
| Normal private inference policy | 4 checks, 4 fake requests |
| Installed old release schema16 → 24 and full rollback | 6 checks, 3 fake requests |
| Previous development schema23 → 24 and full rollback | 5 checks, 3 fake requests |

The schema16 rehearsal used exact copies of the installed private-daily-5ca1951
executables over entirely synthetic state. It verifies retained identity, memory,
receipts and the existing unlimited selected-private policy, default-off new
scopes, old-binary refusal of schema24 and restoration of the complete stopped
original with ACLs. No personal database/credential copy was used in rehearsal.
All temporary instances/services clean up. The eight process reports agree on
the final binaries:

```text
lain.exe  2376b35e258e087452fa1aaff6fd930a9719a8a479726089a81e9a7450582296
laind.exe 93aee6e18d9bbbac925257b001183b48ab3c05fcd993e7f592f8e94da8de98f2
```

Reports and immutable review package stay outside Git under the local
target/visual-chat acceptance directory. Native macOS/Linux, actual CDN/NapCat
image compatibility and real conversational visual quality remain unaccepted.
No personal installation or automatic real-model replay is implied by passing
these gates. `channel show <id>`/the authenticated channel snapshot expose the
latest 128 media states, typed failures and sample metadata without pixel bodies.

## Concrete personal upgrade boundary

Before the separately authorized installation above, F:\Lain-Q1-Acceptance ran
schema16 on private-daily-5ca1951. Its normal private conversation scope remains
bot<bot-qq> ↔ owner<user-qq>. The steps below describe the reviewed operation
and retained rollback boundary; they are not an additional pending approval.

Before installation, review the immutable staged release and fresh diagnostics/
active work. Stop/drain the owned launcher gracefully; never kill accepted Worker
work to make room. Back up the complete stopped instance with native ACLs,
credentials, database/sidecars, config, binaries and launcher. Keep a private
verified file manifest without disclosing credential bodies. Test migration on
the stopped backup copy without model/QQ configuration first. Compare core
identity, permission/budget and delivery state; new scopes remain default off.

Only then install the exact accepted binary pair and enable receive_images and
supports_images in the existing selected configuration. Reuse the existing
SecretRef, endpoint/model and normal private inference decision. This does not
reopen a closed campaign, create autonomous paid replay, enable memory/source/
contact/Worker permissions or start group collection. A live image sent by the
owner remains normal private conversation; any autonomous real-provider test
needs its own explicit bounded decision.

On failed readiness/migration/identity/visual verification: stop the new instance,
retain the upgraded state separately, restore the complete matching stopped
backup/binary/launcher set, verify original schema16 and restart normally. Never
downgrade user_version or copy an upgraded database over the backup. Reboot
autostart and native macOS/Linux acceptance remain unverified.
