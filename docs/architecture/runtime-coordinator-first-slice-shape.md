# Runtime Coordinator: first implementation shape

Status: implemented, 2026-10-04. Local hosting and Linux Docker communication verified; real Hetzner power acceptance remains unverified without a cloud token. A1: register an existing VM and control power/status. B1: enter the Hetzner token in setup and store it on the coordinator. See [verification evidence](runtime-coordinator-first-slice-evidence.md).
Baseline: `feature/Remote-Development` rebased onto local `main`, `a598b7aa`; no branch-only commits or source changes preceded the rebase.

## Agreed target

- A coordinator program can run on a permanently accessible node. Without a configured external coordinator, Orchid uses a local instance of the same program.
- Simulate external deployment on this laptop, preferably in a Linux Docker container.
- The first coordinator responsibility is laptop communication and explicit Start/Stop control of the Hetzner worker.
- Add a top-level Runtime Coordinator perspective. Its front view is a device list with Add device leading to a separate initial setup page.
- Codex, Claude, and further device conditions/controls may eventually belong here; provider setup migration is not prescribed for this slice.

## Proposed scope and assumptions

Register an existing Hetzner Cloud Linux server, observe its provider-reported power state, and request power-on or graceful shutdown. Setup does not create a cloud VM, install `orchid-host`, authenticate an agent provider, or make a machine an execution target.

Propose entering a Hetzner API token through setup and storing it on the selected coordinator. Credentials are write-only through the UI/API and excluded from navigation state, device DTOs, logs, and the ordinary device database. A protected coordinator-owned credential directory is sufficient for this prototype; no general credential product is introduced.

Propose local mode when no external endpoint is configured. An unavailable configured endpoint remains disconnected; it does not silently create a second coordinator. Choosing another coordinator selects its device catalogue; catalogue migration is outside this slice.

The coordinator must be hosted independently of the worker it powers down. Docker on the laptop rehearses that separation, not laptop-off availability.

## Current ownership and change surface

- `src/features/technicalSettings/ExecutionSetupOverview.tsx` combines device lifecycle controls, a modal command editor, provider setup summaries, and inference summaries.
- `src-tauri/src/execution_devices.rs` combines local device records, activity leases, lifecycle programs, an idle scheduler, and Tauri commands. `ExecutionTargetService` constructs it and calls `run_start` from remote readiness preparation.
- `execution_targets` routes existing provider/worktree execution. Its SSH binding is not a cloud power-control connection.
- `productNavigation.ts`, `useProductNavigation.ts`, and `App.tsx` own application surfaces, semantic destinations, and Back/Forward behavior.
- `scripts/build/application.mjs` currently publishes the desktop executable. Local fallback needs its coordinator executable included in retained and packaged outputs.
- Axum, Tokio, Reqwest, Rusqlite, Serde, and UUID are already used by the Rust application. React and existing navigation/modal/style components cover the frontend.

## Suggested ownership

```text
Runtime Coordinator views
  -> RuntimeCoordinatorClient (application port)
  -> Tauri bridge (endpoint selection, local process ownership, local activity guard)
  -> Coordinator HTTP API
  -> Device service / device and operation persistence
  -> Hetzner adapter
  -> Hetzner Cloud API
```

Create `crates/orchid-coordinator` as an independent headless crate with a library and `orchid-coordinator` binary. It owns managed devices, credentials, power operations, and observed provider state. Keep it independent of Tauri, desktop databases, provider runtimes, and `orchid-engine` session ownership.

Its library exports protocol DTOs and the HTTP client consumed by the desktop. Use Axum/Tokio for HTTP serving, Reqwest for requests, and Rusqlite for coordinator-local persistence. A narrow Hetzner adapter is enough; use standard HTTP libraries rather than a custom transport or a new CLI installation dependency.

Keep desktop endpoint settings and local process supervision in `src-tauri/src/runtime_coordinator/`. The frontend receives coordinator identity, hosting mode, availability, and device views through Tauri; it does not manage processes or retain access tokens.

Coordinator device records are authoritative for cloud lifecycle configuration. Existing desktop device identities and activity leases remain authoritative for current execution routing. An explicit local binding can associate an existing execution device with a coordinator device, preserving its ID without copying profiles or creating a second writable lifecycle configuration.

## Proposed visible flow

### Device list

- Header: Runtime Coordinator; compact local/external hosting and connection status; Configure coordinator.
- List: device name, kind, last observed power state, observation time, and current/last operation. Selecting a row opens its detail page.
- Add device opens a full setup page. An empty list explains the supported first connection: an existing Hetzner Cloud Linux server.
- Existing local/legacy execution devices appear with their source identified. This laptop has no cloud power buttons. Linking an existing device replaces its lifecycle controls with the coordinator route rather than adding a duplicate row.

### Initial setup page

1. Show the selected coordinator that will own the device and credential.
2. Enter the Hetzner token; Connect checks it through read-only server discovery.
3. Select an existing server, showing its name, location, address, and power state. Allow a display name override.
4. Where applicable, select the existing Orchid execution-device identity to associate with this server.
5. Add device persists the binding and opens its detail page. Adding it never starts or stops the VM.

Use one page with progressive sections rather than a generic wizard framework. Back returns to the list. Non-secret draft fields survive tab changes within the application; token input stays transient and clears when setup is saved or abandoned.

### Device detail and coordinator settings

- Detail shows provider identity, address, power state, last observation, operation progress/result, and credential configured/missing. Offer Refresh, Edit setup, Start, and Stop.
- Stop means graceful shutdown. No automatic escalation to forced power-off.
- Show power status separately from agent execution setup. Until an existing runtime association exists, display execution setup as not configured; running power state does not imply agent readiness.
- Coordinator settings selects bundled local hosting or an external URL and access token, and tests connection before applying it. The Docker instance is selected through the external mode even though its URL is on localhost.
- Reuse semantic product navigation for list, setup, detail, and coordinator settings. Keep network refresh independent from page navigation.

## Lifecycle behavior

Use a versioned authenticated HTTP API for health/identity, device discovery and registration, device status/editing, and power-operation submission/status. Poll while an operation is active; explicit Refresh covers idle observations. No WebSocket or generic job queue is needed.

Persist an operation ID and intent before submitting to Hetzner; retain its provider action ID when returned. Distinguish coordinator acceptance, provider acceptance, action success/error, and observed power state. A lost submission acknowledgement stays unconfirmed and is not automatically replayed. After coordinator restart, a retained provider action ID permits observation of that action without resubmission.

The desktop Stop command retains the existing active-Orchid-work guard for an associated execution device. This first single-controller slice does not claim knowledge of manual server work or other controllers.

For coordinator-managed devices, remote readiness reports that explicit Start is needed rather than invoking automatic startup. The existing idle scheduler skips these devices. Legacy command-managed devices retain their current behavior; the new Hetzner setup does not expose arbitrary lifecycle programs or idle-shutdown controls.

## Local and Docker deployment

- Bundle the same coordinator executable with the desktop. The local supervisor attaches to its healthy instance for the installation data root, or starts it with loopback binding and installation-owned storage. App/review instances sharing that root must not create competing owners.
- Use Tauri's external-binary packaging support. Extend retained application publication to include the coordinator, so an opened Worktree Review build can use local hosting.
- Add a Linux Dockerfile and Compose definition with a persistent coordinator data volume, loopback-published port, and an access-token secret file. Keep secrets out of image layers and tracked configuration.
- Both deployments expose the same API and run the same device service. The Docker route has separate coordinator storage and survives an Orchid UI restart.
- An actual external endpoint requires HTTPS; loopback HTTP is sufficient for this laptop rehearsal. Network enrollment and multiple-controller authority are deferred.

## Concrete file changes

| Action | Files | Responsibility |
| --- | --- | --- |
| Create | `crates/orchid-coordinator/{Cargo.toml,Cargo.lock,src/lib.rs,src/bin/orchid-coordinator.rs}`; focused `protocol`, `client`, `service`, `repository`, `http`, `credentials`, `hetzner` modules | Independent coordinator, API, durable lifecycle state, and cloud adapter. |
| Create | `src-tauri/src/runtime_coordinator/{mod,domain,local_process,repository,transport,tests}.rs` | Desktop endpoint settings, coordinator process, existing-device associations, and Tauri bridge. HTTP client ownership remains in the standalone crate. |
| Adapt | `src-tauri/Cargo.toml`, `src-tauri/src/{lib,active_app,storage}.rs` | Dependency, composition, endpoint/binding persistence, and commands. Build tooling supplies sidecar configuration without changing the base Tauri config. |
| Extract/adapt | `src-tauri/src/execution_devices.rs` into focused activity/lifecycle modules | Reuse the active-work guard and identities; reject implicit lifecycle calls for linked devices and retain legacy commands separately. Existing execution-target callers inherit these guards. |
| Create | `src/application/runtimeCoordinator/contracts.ts`, `src/infrastructure/runtimeCoordinator/tauriRuntimeCoordinatorClient.ts` | Typed application port and native adapter, separate from worktree/provider targeting. |
| Create | `src/features/runtimeCoordinator/{RuntimeCoordinatorScreen,DeviceList,DeviceSetupPage,DeviceDetailPage,CoordinatorSettingsPage,LegacyDeviceSettings}.tsx` and feature stylesheet | Device-first perspective, page flow, and relocated existing command editor. Extract a hook only where shared loading/state ownership warrants it. |
| Adapt | `src/application/productNavigation.ts`, `src/app/useProductNavigation.ts`, `src/app/App.tsx`, `src/bootstrap/productApplicationComposition.ts` | New typed destinations, tab, history, client injection, and surface composition. |
| Extract/remove | `src/features/technicalSettings/ExecutionSetupOverview.tsx`, `TechnicalSettingsScreen.tsx` | Retain provider/inference summaries as Agent setups; move device controls and legacy command editing into Runtime Coordinator. The former Devices navigation key remains for those agent summaries. Keep Codex/Claude setup screens here for now. |
| Adapt/create | `scripts/build/application.mjs`, new `scripts/build/coordinator.mjs`, build tests, package scripts | Build/stage the coordinator and publish it with the application. Retain existing cache behavior. |
| Create | `deploy/runtime-coordinator/{Dockerfile,compose.yaml}`, deployment guide | Reproducible simulated external deployment, storage, and credential mounting. |
| Update | Relevant existing device/coordinator architecture documents | Mark older Technical Settings ownership and excluded-coordinator scope as superseded by this bounded slice. |

Avoid broad changes to `orchid-engine`, provider registrations, Capability Profiles, worktree snapshots, or session routing. New coordinator-managed power control is the reason for the device-service extraction; a general execution-architecture rewrite is not part of this plan.

## Validation and execution order

1. Establish coordinator protocol/service/persistence and narrow Hetzner adapter. Test read-only discovery, action status, rejected credentials, duplicate submission identity, and unconfirmed outcomes against a controlled HTTP peer.
2. Add the desktop bridge/local hosting and build publication. Verify local and Docker endpoints serve the same contracts, persistent state survives restart, and configured external failure does not start local hosting.
3. Build the list/setup/detail/settings flow and relocate device controls. Test Add -> Connect -> select -> save -> detail, Back/Forward, tab return, per-device errors, and Start/Stop projection.
4. Integrate existing-device association and active-work guards. Test that linked devices cannot be implicitly started, idle-stopped, or stopped during current Orchid-managed activity; preserve unrelated legacy behavior.
5. Run affected frontend tests, frontend build, relevant Rust tests/checks, build-tool checks, and `git diff --check`.
6. Launch an isolated built Orchid instance and review screenshots/interactions using the repository application-control tools. Exercise setup and read-only real Hetzner discovery, then verify explicit Start/Stop on the idle intended worker and observe actual provider state. Keep credentials out of evidence.

Live credentials are needed for real Hetzner acceptance. A mock-provider result, Docker reachability, or power state alone does not establish real cloud control or agent execution.

## References

- Existing ownership: `execution_devices.rs`, `execution_targets/mod.rs`, `technicalSettings/ExecutionSetupOverview.tsx`, `productNavigation.ts`, and `scripts/build/application.mjs`, inspected at `a598b7aa`.
- [Hetzner Cloud API](https://docs.hetzner.cloud/reference/cloud): server discovery, power-on/shutdown, asynchronous actions.
- [Axum](https://docs.rs/axum/latest/axum/) and [Reqwest](https://docs.rs/reqwest/latest/reqwest/): reuse the application's existing HTTP stack.
- [Tauri external binaries](https://v2.tauri.app/develop/sidecar/): bundled local coordinator.
- [Docker Compose secrets](https://docs.docker.com/compose/how-tos/use-secrets/): file-mounted deployment access token.
