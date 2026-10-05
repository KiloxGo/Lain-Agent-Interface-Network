# ADR 002: Same-origin Web assets and explicit in-memory authentication

Date: 2026-10-05. Status: accepted; implemented in Phase 4.

## Context

The Kernel already offers bearer-authenticated HTTP/SSE, atomic snapshots and
durable paginated history. A Web client needs to load before authentication and
must not duplicate worker ownership or persist the owner's long-lived token in
browser storage. Phase 4 permits a temporary in-memory credential flow.

## Decision

Serve built React/TypeScript/Vite assets from an optional `LAIN_WEB_DIR` on the
Kernel's existing loopback listener. Static assets are public; every API route
retains the same bearer requirement as CLI/TUI. Vite proxies API requests during
local development. No CORS relaxation, WebSocket or mandatory reverse proxy exists.
The first deployment supports origin-root hosting.

Require deliberate token entry in a password field. Clear the field on submission;
hold the token only in a page-lifetime Client, send it only in bearer headers and
discard that client on disconnect. Refresh requires authentication again. No token
in URLs, browser stores, bundles or normal logs. A selected public session ID may
appear in the URL hash. Fetch streaming supports authentication and the existing
SSE snapshot/history recovery contract.

Do not introduce a cookie layer merely to conceal the unchanged long-lived token
without a complete session policy. The longer-term browser design should exchange
owner credentials for short-lived opaque server sessions, with TLS, HttpOnly,
Secure, SameSite cookies, expiry/revocation, trusted Origin checks and CSRF defenses
for mutations. Server-side session state needs an explicit owner and rotation/
logout behavior. Establish and review that deployment boundary before public
Internet exposure; it is not implemented by this milestone.

## Consequences

One daemon serves browser and terminal clients with identical worker privileges.
Headless use remains possible, and static delivery has no worker-lifecycle side
effects. No persisted browser credential can silently survive refresh. The interim
flow is less convenient and remains vulnerable to compromised first-party code,
extensions or transport; CSP and escaped text reduce, but do not eliminate, that
boundary. Loopback binding remains enforced. SSH tunneling is a supported interim
transport; public Internet readiness is not claimed.
