# Optional cloud QQ transport

Verified upstream deployment instructions: 2026-11-05,
[NapNeko/NapCat-Docker](https://github.com/NapNeko/NapCat-Docker).
This template does not activate QQ or grant memory/contact permission by itself.

Run NapCat beside the existing cloud Kernel. The PC is only a terminal/Worker
client; a tunnel to a PC-hosted NapCat does not provide PC-independent residency.
The container owns the QQ login/session and OneBot transport, not Lain's Actor,
memory or database. No provider credential belongs in this container.

Prepare a private deployment directory and a separate service account/state root.
Pull the official image deliberately, inspect its immutable repository digest,
then set `NAPCAT_IMAGE` to `mlikiowa/napcat-docker@sha256:<inspected digest>`.
The template requires explicit `NAPCAT_UID`, `NAPCAT_GID`, `NAPCAT_STATE` and
`NAPCAT_ACCOUNT` (the intended Lain QQ account for quick login after restart).
Keep this operator `.env` outside Git with mode 0600. Updates are manual; retain
the selected digest and state rather than automatically pulling `latest`.

Create `qq`, `config` and `plugins` below the private state root for the dedicated
UID/GID. Initialize `config/webui.json` and `config/onebot11.json` with different
random management/OneBot credentials before the first launch. Never use an empty
OneBot token or the Kernel administrator credential. They listen on all interfaces
**inside the isolated container**; host publishing is restricted to literal
127.0.0.1. Do not add privileged mode, host networking or a Docker socket mount.
Preseed `config/napcat.json` with `fileLog: false` and `consoleLog: false`.
Native deployment on 2026-11-05 showed that pre-login output still includes
auth-bearing management/QR URLs despite these settings. The container therefore
uses Docker's `none` log driver, preventing that stdout/stderr from being retained.
Obtain login/status through the authenticated management API and container state
instead. Do not enable persistent console logging for troubleshooting.

Validate the rendered config and run only this project:

```text
docker compose --project-directory <private-deployment-directory> config --quiet
docker compose --project-directory <private-deployment-directory> up -d
docker compose --project-directory <private-deployment-directory> ps
```

Access WebUI only through a pinned-host authenticated SSH forward, using a free
local port (the PC's older NapCat may already use 6099). Login QR and auth-bearing
URLs are transient credentials: keep them outside Git and ordinary logs. Verify
the actual logged-in account through authenticated status before connecting the
Kernel or enabling reply. Disable the old binding's reply grant before switching,
preserve its records and start/rollback instructions, and never clone the old
personal database into the cloud. A QR displayed is not proof of login.

Start the new binding with replies disabled, verify the exact account is Online,
then perform the explicitly authorized reply grant. Persist `allow_reply: true`
at that enable step: leaving it false revokes an already enabled binding on the
next Kernel startup. True does not override an independent database revocation.
Back up the final enabled configuration and verify an idle restart retains the
same binding/conversation/grant before declaring restart readiness.

Configure the Kernel's existing NapCat adapter with its own private config and
SecretRef environment file. A systemd override may select these files; retain the
existing provider/budget/administrator configuration. Inspect unknown or in-flight
delivery state before a restart. Private replies do not enable group collection,
long-term memory, autonomous reading or proactive contact.

Resources: memory/swap 1536 MiB, 1.5 CPUs, 512 processes, core dumps disabled and
container stdout/stderr logging disabled. These limits do not bound QQ's persistent
media/session directories or the writable image layer. Monitor their measured
size; do not blindly delete session data or claim Kernel backups include it.
The two bind-mounted login/configuration trees need a separately reviewed backup
procedure. Plugins are not installed by this template.

### Optional temporary picture retention

`scripts/napcat_cache.py` is a separate Linux maintenance helper. Its default is
read-only preview. It visits only the top level of `<state-root>/NapCat/temp`,
accepting known image suffixes owned by the selected non-root QQ UID with one
hard link. It never traverses account databases, avatars, native media directories
or a returned `get_image.file` path. Symlink components/files and unexpected
owners are refused or skipped. It does not call the broad OneBot `clean_cache`.

Defaults: expire pictures after 24 hours; when over the soft 512 MiB target,
remove oldest eligible pictures at least one hour old. Scan at most 10,000
directory entries and delete at most 256 pictures per run. Incomplete inventory
causes no deletion. Fresh, unknown or protected files can leave the target unmet;
the metadata-only report says so. This bounds work, not all QQ disk consumption.
Kernel picture frames separately retain their original ten-minute expiry and
64 MiB global limit; the helper does not change conversation or memory policy.

Explicit installation maps `cache-maintenance.service` to
`/etc/systemd/system/lain-napcat-cache.service` and `cache-maintenance.timer` to
`/etc/systemd/system/lain-napcat-cache.timer`. Copy the tested helper to
`/srv/lain/ops/napcat_cache.py` with root-only write permission. Replace
`REPLACE_QQ_UID` with the verified dedicated QQ process UID, verify both units,
run a dry preview, then start the service once and inspect its count-only report
before enabling the timer. The timer runs hourly with jitter; the service has a
60-second/96 MiB bound, no network, and one writable temporary directory. No
Kernel/container restart is needed. Disable only `lain-napcat-cache.timer` to
stop scheduled cleanup.

Each selected file is quarantined and rechecked before unlinking. A racing
replacement is restored without overwriting a newly created name. A nonempty
`.lain-media-prune` directory indicates interrupted/contested work: subsequent
maintenance fails closed. Disable the timer, inspect the exact retained object
and original name manually; never delete this directory recursively or treat an
incomplete report as permission to replay the operation.

Stop/rollback with this project's `stop`, not global Docker cleanup. Preserve
state, disable cloud replies, and verify cloud sends are quiescent before restoring
the old local binding. `down -v`, broad pruning and removal of other applications
are not part of this procedure.
