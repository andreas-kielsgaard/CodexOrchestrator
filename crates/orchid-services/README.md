# Independent Orchid services

`orchid-server` stores synchronized data and forwards addressed commands. `orchid-host` owns provider processes, queues, worktrees, MCP servers and local workflow execution. Run them as separate processes with separate data directories, even on one machine. Neither service requires the desktop UI to stay open.

The desktop remains a complete local host. Configure its optional connection under **Technical settings → Synchronization server**, then restart it. Disconnecting the server leaves local execution running; accepted host work is independent of the routing connection.

## Build and start

From the repository root:

```powershell
npm run build:services
npm run test:services
```

The build wrapper reports its output directory. For Linux, `scripts/package-orchid-services.ps1 -Destination <archive.tar.gz>` packages the workspace source and lockfile without databases or provider credentials. Extract it and run:

```sh
cargo build --locked --release -p orchid-services --bins
export ORCHID_SERVICE_TOKEN='<shared random token>'
./target/release/orchid-server --data-dir /absolute/orchid/server --listen 127.0.0.1:4480
# In a separate process:
./target/release/orchid-host --data-dir /absolute/orchid/host --listen 127.0.0.1:4481 --server http://127.0.0.1:4480
```

Omit `--server` for a standalone host. `ORCHID_DEVICE_NAME` sets its advertised name. Device identity is durable in its database. Do not clone an execution host's database into another running host: synchronization copies conversations without copying execution authority. Instance locks reject concurrent processes or different service roles sharing a data directory.

The services use bearer authentication and default to loopback. Connect devices over a private network, or terminate HTTPS in a reverse proxy that supports WebSockets. The shared token grants access to all connected devices; this initial implementation has no separate users or per-device authorization. Keep host port 4481 local. Configure the same server URL/token on each attached device.

`scripts/deploy-orchid-host.ps1` builds and installs both binaries over SSH; it does not start them. Its default install directory is separate from the legacy SSH host. To install Linux user services, create `~/.config/orchid/services.env` with `ORCHID_SERVICE_TOKEN=...`, restrict it to mode 600, then run:

```sh
sh scripts/install-orchid-services.sh /absolute/service/bin 127.0.0.1:4480 http://127.0.0.1:4480
```

This starts two independent user units. Use a reachable private listen address for remote clients. Provider environment settings can go in `~/.config/orchid/host.env`, which only the host unit reads. The units must have access to provider executables in their PATH. Enable user lingering separately if services must survive logout.

## Configure a work host

Install/login to the provider on that host using its normal tools. Native provider homes and credentials stay there. The host's loopback API can register an existing Codex or Claude home:

```sh
curl --fail-with-body http://127.0.0.1:4481/v1/commands \
  -H "Authorization: Bearer $ORCHID_SERVICE_TOKEN" -H 'Content-Type: application/json' \
  -d '{"method":"local.registerProvider","input":{"provider":"codex","home":"/absolute/provider/home"}}'
```

The result supplies a `configurationRef`; registration is an observation, not a guarantee that a future launch succeeds. `local.registerProvider` is deliberately unavailable through the router. Claude also accepts an optional `executable`. Jev uses its existing environment configuration.

Configure a Capability Profile for that device through the desktop, or pass a complete `CapabilityProfile` to `device.saveProfile` with `{"profile": ..., "default": false}`. This uses the existing profile contract; no provider is selected automatically. `device.profiles` lists configured choices.

Register repositories with `device.registerRepository`, supplying `path` on the addressed host and optionally the existing logical `repositoryId`. Linking that ID makes laptop/server checkouts one logical repository without assuming their filesystem paths match. Existing conflicting registrations are rejected. `device.worktreeChoices` and `device.targets` return explicit launch targets.

## API boundaries

Every request needs `Authorization: Bearer <token>`. Requests and responses use the shared Rust/Serde camelCase contracts. Mutations go to the addressed host; the server does not queue work for offline devices.

| Server endpoint | Purpose |
| --- | --- |
| `GET /v1/device-directory` | Stored device/provider descriptions plus current connection status |
| `GET /v1/devices` | Currently connected device IDs |
| `POST /v1/devices/{id}/commands` | `{ "method": "...", "input": {...} }` addressed host command |
| `GET /v1/sessions`, `GET /v1/sessions/{id}` | Synchronized session summaries and history |
| `GET /v1/sessions/{id}/events/{event}` | Full persisted event details |
| `GET /v1/sessions/{id}/continuation` | Reachability, fork requirement and lineage |
| `POST /v1/sessions/{id}/commands` | Route a session mutation to its recorded owner; read stored queue/preparation/interactions while offline |
| `POST /v1/forks` | `{sourceSessionId, forkId, target}`; copy complete retained turns to an explicit new target |
| `GET /v1/definitions`, `GET /v1/skills`, `GET /v1/skills/{id}` | Canonical definitions, skill catalogue and package files |
| `POST /v1/definition-edits` | One immediate attempt to change a definition through its online master |

Host commands are enumerated in `orchid-core/src/agent_sessions/client.rs` and `orchid-core/src/devices/operations.rs`. These include send, steer, respond, cancel, queue edits, target transitions and host Git/worktree operations. `send` uses `PreparedMessageInput`; `sendProduct` uses the same preparation path with application provenance. Both require a stable `submissionId`. Target operations require `operationId`; compaction requires `submissionId`; forks require `forkId`. Reuse the same ID and payload after a lost reply. A successful reply means acceptance, not completed provider execution.

The direct host API provides `POST /v1/commands` and session reads without the server. Host connections reconnect automatically. Synchronization polls committed changes; clients can poll `/v1/changes?after=<cursor>`. There is no browser/Android client in this change.

## Storage and recovery

- Runtime histories, events, attachments, queue observations and fork lineage synchronize as owner-authored snapshots. Importing one never starts its runtime or queue.
- The desktop can retain all copied histories or cache them on demand. Retained mode is the default. Cache clearing removes copied histories, not locally owned work. The UI reports known/full-copy counts; missing cache content requires a reachable server.
- Definitions and skill packages are currently fully retained on attached devices. Separate definition/skill cache controls remain future work. Workflow definitions synchronize; workflow instances and progression remain local to the execution host.
- Definitions use creator-master replacement. Non-master offline edits remain local until canonical synchronization overwrites them. They are not replayed later. Online accepted edits update the master's Orchid-owned copy. Imported skill origins are refreshed only when source bytes change; edits never write back to the external source folder.
- Worktrees, provider credentials, native configuration homes and arbitrary MCP environment configuration are not synchronized. A destination still needs its own provider setup. Legacy Harness configurations remain local; portable Capability Profiles, identities, skills and workflow designs synchronize.
- An explicit online worktree migration transfers queue authority only after the source has stopped owning it. Pending delivery is retried on reconnection. Unreachable-host continuation creates a fork and requires a new target; it does not claim to recover unsynchronized files or turns.
- Back up each retained data directory using a consistent SQLite backup or with its process stopped. Retained copies provide independent conversation access, not a backup of Git worktrees or credentials.

Legacy SSH profiles still use their existing execution path. The old engine binary is now named `orchid-ssh-host`; its smoke scripts are under `scripts/legacy`. New server-connected devices use `orchid-host`. Existing installations/configuration are not rewritten automatically. Desktop branch-management menus and experimental voice remain local-facing compatibility surfaces.

## Verification

Core tests cover persistence, definition replacement, skill packages, fork provenance, cache behavior and ownership transfer. Service tests cover offline local commits, router restart, retained history, authentication and an actual Jev adapter against a controlled HTTP endpoint. They do not spend provider credits.

`python scripts/verify-orchid-service-processes.py --help` describes the isolated separate-process check. `scripts/remote-host-smoke.mjs` probes an installed server/device read-only by default. Set `ORCHID_SERVER_URL`, `ORCHID_SERVICE_TOKEN` and `ORCHID_DEVICE_ID`. Supplying both `ORCHID_SMOKE_TARGET_FILE` and `ORCHID_SMOKE_PROMPT` explicitly opts into a real provider turn, duplicate-submission check and server-retention check.
