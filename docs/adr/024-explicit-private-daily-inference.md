# ADR 024: Explicit private daily inference mode

Date: 2026-10-28 (Asia/Shanghai). Status: implemented; native validation and local
installation evidence recorded in DEVLOG. User explicitly chose no count/time
ceiling for normal existing private QQ replies, superseding the proposed new
60-request/six-hour test allowance. This does not reopen an old campaign.

An authenticated CAS decision may select either a finite global inference limit,
or `unlimited_private_channel` for one existing binding, or stop. The private mode
has no cumulative request cap or expiry. It permits only ordinary private turns
of the selected channel, at both reservation and adapter entry. Unbound chat,
other private bindings, reading and sharing remain denied by this decision.
Reply/source/memory/contact/Worker policy remains independently enforced. It is
not an idle model loop or an unlimited Worker/image-processing budget.

Per-turn requests, tools, token and timeout limits remain intact. All attempts,
including unbudgeted daily requests, retain durable usage/Unknown recovery.
Historical finite budget IDs and spent reservations are unchanged. Restart keeps
the mode; stop or a newer decision invalidates prepared attempts. Re-enabling an
old finite budget cannot refund its spent requests.

The wire adds an optional field to existing decision snapshots and typed requests;
no SQLite schema migration is needed. Old snapshots default to no private daily
scope. An old daemon sees an enabled daily decision with `limit:null` as stopped,
so rolling back without restoration fails closed. Before a local binary upgrade,
stop normally and take an ACL-preserving complete backup; retain old binaries and
the full snapshot for rollback. Do not deploy unrelated in-progress migrations.

`lain inference private-daily <channel> --revision <n>` opts in explicitly;
`lain inference show` exposes `private_conversation_only`, the selected channel,
and no finite remaining-request number. `inference stop --revision <n>` stops it.
API POST `/v1/inference/budget` accepts
`{"expected_revision":0,"limit":null,"unlimited_private_channel":"<ChannelId>"}`.
Both finite and daily fields set together are rejected.
