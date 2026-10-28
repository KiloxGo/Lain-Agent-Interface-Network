# ADR 027: Current private pictures and QQ expressions as conversation input

Date: 2026-10-28. Status: implemented; deterministic Windows acceptance passed;
real provider/QQ visual acceptance and personal installation pending.

## Problem and scope

The user explicitly selected viewing pictures and QQ expression/sticker messages
in ordinary conversation. The independent QQ knowledge service already has a
vision model, but the conversation adapter rejects every non-text segment. Its
vision description queue and finite lifetime budget are not the chat path.

Implement actual visual input to the existing conversational Responses model,
not a caption-only workaround or another model/planning loop. Preserve mixed
text/media order, image-only input, ordinary/custom/market stickers, standard QQ
faces and bounded animated-image samples. Unsupported/unavailable media must
produce explicit availability evidence, never an invented picture or silent loss.
Ordinary text conversation remains available. Outbound image generation/sending,
voice/video, broad history collection and new semantic-memory authority are not
part of this request.

## Boundaries

- Continue exact verified bot/private-peer filtering and revocable reply scope.
  Receiving a current picture is not permission to read arbitrary device files,
  conversations, groups, quoted-message histories or instructions in pixels.
- Add an explicit default-off received-image setting on the selected NapCat
  binding and declared image-input support on the selected Actor model. Enabling
  current private pictures does not enable archive/memory/contact/Worker grants.
- Use the selected existing visual-capable model in the ordinary private turn;
  calls retain the same Kernel inference ledger, per-turn limits and no retry.
  Image bytes enter provider JSON only inside the Responses adapter. The
  independent knowledge service retains its separate ingestion/vision budget.
- Download only current event media through the verified adapter. QQ file/URL
  references are transient; never put signed URLs, local paths or transport
  credentials in Input text, model context, logs or API history. Restrict CDN
  hosts, upgrade legacy CDN HTTP to HTTPS, pin public DNS results, disable
  redirects/proxies/retries and bound time/body/decode resources. Kernel must not
  open a path from `get_image` or an image segment. The verified OneBot socket
  can return bounded `get_image.base64` bytes; this is not permission to open its
  accompanying `file`/`url` device path. Only opaque current-event references
  are submitted to `get_image`, never absolute paths or URLs.
- Standard QQ faces use ID/name metadata with attributed mapping provenance;
  unknown IDs remain unknown. They do not require a model image call. Picture
  stickers use actual decoded pixels; a platform-provided summary is a label,
  never a substitute for inspecting the picture.

## Storage, preparation and continuation

Keep normalized media metadata in Incoming and original text separate from
adapter-generated markers. Lain-owned media identities point to a private
operational media store, with compressed normalized frames outside JSON/event
history. Retain frames for at most the existing ten-minute reply window. No raw
CDN reference or whole source event is durably copied. Migration 024 enables no
new binding/provider capability and preserves old inputs/turns/deliveries.

Commit receipt/order before fetching. A bounded owned preparation task runs off
the WebSocket receive/send loop, preserving new-input invalidation while media
loads. Preparation completion/failure wakes existing channel/Actor consumers;
queued media blocks its batch only until a finite deadline. Text following an
image remains ordered and may join the same bounded batch. Restart/shutdown do
not blindly repeat interrupted external reads; unavailable media gets an honest
status. Duplicate events reuse their original identities and preparation state.

Decode supported JPEG/PNG/GIF/WebP with explicit dimensions/allocation/work bounds.
Animated formats supply a bounded sequence of sampled composited frames and
actual sample positions/truncation metadata; do not claim to have watched beyond
those samples. Normalize size and bound aggregate provider payload. A limit,
unsupported/flash image, download failure or missing model capability remains
inspectable and must not masquerade as a successfully viewed image.

Only current/recent same-binding conversation media within its original lifetime
may be supplied, under the original reply epoch. Images and their relevant text
stay associated by input/segment identity. On each continuation recheck the
selected media state, reply scope and current input. Revocation, expiry, withdrawal
or new input invalidates stale prepared speech through adapter entry, response,
commit and send. Later history must not revive withdrawn/expired visual content
through old assistant/tool descriptions. Raw frames are pruned by existing
channel expiry timers and startup recovery, without idle model polling.

Visual input is untrusted attributed evidence. Preserve existing reviewed Worker
actions and do not silently promote image-derived interpretations into ordinary
quoted-text memory. Ordinary clearly quoted current-user corrections retain their
own source semantics. The model should respond naturally to a sticker's social
intent, not mechanically caption every picture or invent participation. When a
picture cannot be read, say what is unavailable rather than silently ending.

## Required evidence

1. Native parser/adapter tests: exact accounts, image-only, mixed order, multiple
   pictures, standard faces, custom/market stickers, animation and unsupported
   status; no raw signed URL/path/credential in stored or model-facing metadata.
2. Preparation lifecycle: actual bounded bytes/frames, duplicate/order, concurrent
   text and reply transport, timeout, decode/SSRF limits, revoke/withdraw/expiry,
   restart and cleanup; no hidden retry or preparation-driven model calls.
3. Actual Actor/Responses tests: pixel data reaches `input_image`, associated
   user text remains, continuation/reconsideration freshness and old visual-body
   suppression, natural-expression instructions and honest failure behavior.
4. Native process acceptance using real local HTTP/WebSocket protocols and
   generated static/animated fixtures, plus regression, fmt and strict Clippy.
5. A concrete verified build and stopped whole-instance upgrade/rollback plan
   before personal installation. Keep native macOS/Linux and real provider/QQ
   acceptance separate. Finish the goal only when the requested conversational
   picture/sticker capability has actual end-to-end evidence on its intended path.

OneBot segment/API documentation and the installed NapCat 52230 asset mapping were
inspected on 2026-10-28. Upstream references:
[segments](https://github.com/botuniverse/onebot-11/blob/master/message/segment.md),
[get_image](https://github.com/botuniverse/onebot-11/blob/master/api/public.md#get_image-获取图片).
Installed NapCat 52230 source was also checked: market expressions are emitted
as `image` with `emoji_id` and a `gxh.vip.qq.com` URL; `get_image` can return a
CDN URL or, with `enableLocalFile2Url`, base64 bytes alongside local paths.
The factual 275 QQ face ID/name pairs come from its qface asset (SHA256
`9fee278a09655cbe79ef2e45c83487dc123b1721f1e0025fe5809edf17d80044`).
These are dated adapter facts, not a general OneBot guarantee.
