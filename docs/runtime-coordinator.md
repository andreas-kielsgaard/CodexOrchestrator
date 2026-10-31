# Runtime Coordinator

Open the Runtime Coordinator tab to view devices. Add device connects an existing Hetzner Cloud Linux server, using a Cloud API token with read/write access for power actions. Connect discovers servers; Add records the chosen server without changing its power state. Start powers it on. Stop requests graceful shutdown. Refresh observes power and action status.

Optional association with an existing execution device keeps its SSH/provider setup and active-work guard. Associated devices require explicit power controls and are excluded from automatic start and idle shutdown. A powered-on machine still needs its existing agent setup to execute work. Codex and Claude setup remain in Technical Settings.

The desktop normally starts or attaches to its bundled coordinator on loopback. Coordinator data and protected credential files live under the installation's application-data `runtime-coordinator/local` directory. The process continues independently of the desktop window. External mode selects a separate coordinator catalogue. A disconnected configured endpoint never triggers local takeover.

## Linux Docker rehearsal on Windows

Run Docker Desktop's Linux engine. From the repository root, create an access token outside tracked configuration:

```powershell
New-Item -ItemType Directory -Force .dev/coordinator | Out-Null
$coordinatorToken = [Guid]::NewGuid().ToString('N') + [Guid]::NewGuid().ToString('N')
$coordinatorTokenPath = Join-Path (Get-Location) '.dev/coordinator/access-token.local'
[IO.File]::WriteAllText($coordinatorTokenPath, $coordinatorToken)
$env:ORCHID_COORDINATOR_TOKEN_FILE = $coordinatorTokenPath
docker compose -p orchid-coordinator -f deploy/runtime-coordinator/compose.yaml up -d --build
```

In Configure coordinator, select External coordinator and choose **Connect to laptop Docker**. Orchid discovers the running Compose deployment by its project/service labels, checks its loopback-published port, reads its mounted access token through Docker, and tests the authenticated connection before saving it. The token stays in native code and protected storage. This action connects an existing deployment; it does not build or start a container.

For another coordinator, enter its HTTPS URL and coordinator access token, then Test and save. This access token authenticates Orchid to the coordinator; the Hetzner token is entered later in Add device. Keep both out of chat, logs, and screenshots of plain text.

The named volume retains devices, operations, and cloud credentials across container restarts. `docker compose -p orchid-coordinator -f deploy/runtime-coordinator/compose.yaml down` stops deployment and retains its volume. Reuse the same secret-file environment variable for subsequent commands.

For a real remote node, host independently of the worker it controls and provide HTTPS through a reverse proxy. This rehearsal verifies process/network/storage separation on the laptop; it does not provide availability while the laptop is offline.

## Build and verification

`npm run build` builds and retains the desktop plus coordinator executable; `npm run build:tauri` also packages the sidecar. `npm run dev:tauri` prepares the coordinator before launching development. `npm run test:coordinator` exercises the authenticated HTTP contract and lifecycle persistence against a controlled provider.

Worktree Review uses the selected source revision's build recipe, which publishes both executables together. A Worktree Review host built before this correction must be updated once; its older embedded publisher only retained the desktop executable. Rebuilding with that old host cannot repair local hosting. Older source revisions predating the build entry point remain supported by the shipped legacy recipe.

New Worktree Review instances keep their own coordinator settings and local state. Review seeding excludes the controller's coordinator directory and its device associations; it does not copy live process endpoints, cloud credentials or a local coordinator identity. Connect the review instance to Docker explicitly to use that deployment's shared device catalogue.

Submission intent and provider receipt are persisted together with the device projection. Repeating an operation ID retrieves its recorded result. Restart observes a retained provider action without replaying it. If acknowledgement is lost, the operation stays unconfirmed until Refresh observes the requested power state; this observation is shown separately from provider-confirmed success.

The first slice registers existing VMs. It does not provision a VM, install the host program, configure agent accounts, transfer a catalogue between coordinators, or arbitrate multiple controllers.
