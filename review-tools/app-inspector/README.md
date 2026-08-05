# Provisional application review companion

This development/review-only CLI observes an explicitly identified Codex Orchestrator instance. It
does not add a Tauri command, production route, driver permission, or orchestration action.

## Explicit desktop interaction companion

`interact-app.mjs` is a separate development-only click transport for a named Windows instance.
It requires both the exact executable path and PID, validates its client coordinates directly in
the PowerShell boundary, and requires the target HWND to belong to that process or a live
descendant. It never foregrounds the window and has no clipboard authority. A receipt proves only
that the target acknowledged the explicit mouse-down and mouse-up messages, so retain a separate
native-window or SQLite observation for every product, provider, or orchestration claim.

```powershell
node review-tools/app-inspector/interact-app.mjs click --exe "C:\path\to\codex-orchestrator.exe" --pid 19760 --x 470 --y 760 --out "C:\path\to\interaction-receipt.json"
```

Coordinates must be from `0` through `32767` and fall in the selected main window's client area.
This tool is for explicitly authorized development/review interaction only; it does not become an
application transport or infer that any requested workflow stage occurred.

For an isolated development instance deliberately launched with a loopback WebView2 debugging
port, `webview-control.mjs` can instead type into or click a CSS selector without foregrounding the
window. It requires the exact owner executable and PID; the listener for the requested port must
be exactly one descendant process whose command line declares that port. It then requires one exact
page target URL and writes a redacted dispatch receipt. The debugger port is a development-only
launch choice, never a production application transport.

## First snapshot

Run from any PowerShell directory:

```powershell
node --no-warnings "C:\Users\user\Documents\Code Projects\Codex Orchestrator\review-tools\app-inspector\review-app.mjs" inspect --workspace "C:\Users\user\Documents\Code Projects\Codex Orchestrator" --exe "C:\Users\user\Documents\Code Projects\Codex Orchestrator\src-tauri\target\release\codex-orchestrator.exe" --instance "sprint-6-review" --status-url "http://127.0.0.1:41415" --out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-before.json" --format human
```

The command matches the process by exact executable path, captures its window without clicking it,
hashes the executable, reads Git/product identity, probes the development status endpoint, and
opens active-v3 SQLite with both `readOnly=true` and `PRAGMA query_only=ON`. It excludes submitted
message text, raw runtime payloads, and credentials from the state summary.

## Wait for a human action

Use one active command when Review Coach asks the human to perform an action. This example watches
both the real window-render hash and the read-only durable-state fingerprint for PID 19760:

```powershell
node --no-warnings "C:\Users\user\Documents\Code Projects\Codex Orchestrator\review-tools\app-inspector\review-app.mjs" wait --workspace "C:\Users\user\Documents\Code Projects\Codex Orchestrator" --exe "C:\Users\user\Documents\Code Projects\Codex Orchestrator\src-tauri\target\release\codex-orchestrator.exe" --pid 19760 --instance "sprint-6-review" --app-data-dir "C:\Users\user\AppData\Roaming\dev.codex-orchestrator.app" --status-url "http://127.0.0.1:41415" --evidence-root "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review" --condition either --poll-ms 500 --stable-observations 3 --timeout-ms 300000 --before-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-before.json" --after-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-after.json" --comparison-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-comparison.json" --human-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-summary.txt" --out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-result.json" --format human
```

The wait captures its baseline unless `--before <snapshot.json>` supplies one. It exits `0` only
after the selected changed fingerprint repeats stably, `2` on timeout, `130` on Ctrl+C/SIGTERM,
and `1` on invalid input or setup failure. It always retains a complete final snapshot, JSON
comparison, and readable summary after a bounded wait outcome.

`visual` observes the whole native window render, `durable` observes the summarized SQLite state,
and `either` accepts either (preferring a durable trigger when both change). Stable repeats reduce
transient-frame and render-jitter false positives; they cannot prove which control changed or that
a requested semantic action completed. A steady caret or animation can still satisfy a visual wait,
and state outside the inspector's summarized tables cannot satisfy a durable wait.

## Detached wait

Launch the watcher and end the Review Coach turn:

```powershell
node --no-warnings "C:\Users\user\Documents\Code Projects\Codex Orchestrator\review-tools\app-inspector\review-app.mjs" launch-wait --workspace "C:\Users\user\Documents\Code Projects\Codex Orchestrator" --exe "C:\Users\user\Documents\Code Projects\Codex Orchestrator\src-tauri\target\release\codex-orchestrator.exe" --pid 19760 --instance "sprint-6-review" --app-data-dir "C:\Users\user\AppData\Roaming\dev.codex-orchestrator.app" --status-url "http://127.0.0.1:41415" --evidence-root "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review" --condition either --poll-ms 500 --stable-observations 3 --timeout-ms 300000 --cancel-file "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\watcher.cancel" --before-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-before.json" --after-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-after.json" --comparison-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-comparison.json" --human-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-summary.txt" --out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\wait-result.json" --watcher-log "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\watcher.log" --launch-out "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\watcher-launch.json" --format human
```

The launch response includes the detached watcher PID and every evidence/log path. Request graceful
cancellation with this exact command; cancellation finalizes evidence and never runs the callback:

```powershell
New-Item -ItemType File -Path "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-review\watcher.cancel" -Force
```

After the PID exits, remove only `watcher.cancel`, or remove the explicit evidence directory if none
of its review evidence is needed. `Stop-Process -Id <watcherPid>` is emergency-only because forced
Windows termination can prevent final evidence from being written.

On stable completion, the watcher atomically writes and syncs before/after snapshots, comparison,
human summary, and wait result. Timeout remains `2` and cancellation `130`.

Desktop wake is intentionally disabled. Live use proved that `codex exec resume <task-id> <prompt>`
starts a separate hidden CLI turn; it does not queue or surface a turn in the existing desktop task,
and that hidden agent can recursively arm another watcher. `--callback-spec` is therefore rejected
before inspection or child launch. The installed Windows desktop app exposes its task app-server as
a private stdio child, not as a supported external endpoint. Until the desktop host provides a
documented authenticated send-to-existing-task transport, the watcher can finalize evidence but
cannot resume Review Coach. Return to the desktop task manually after the action.

After a human action, retain another snapshot by changing only the output name, then compare:

```powershell
node --no-warnings "C:\Users\user\Documents\Code Projects\Codex Orchestrator\review-tools\app-inspector\review-app.mjs" compare --before "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-before.json" --after "C:\Users\user\Documents\Code Projects\Codex Orchestrator\.dev\review-app-inspector\sprint-6-after.json" --format human
```

JSON is the default output. Run `node ...\review-app.mjs --help` for all explicit instance and
storage inputs.

## Truth boundary

- Process, window title, render, file hashes, Git state, endpoint responses, and read-only SQLite
  rows are observed. SQLite rows are additionally labelled `recorded` evidence.
- A source-to-executable relationship is inferred only from path containment; the binary does not
  embed its producing commit.
- The already-running production WebView2 process has no debugging attachment endpoint. Windows UI
  Automation exposes its shell but not the semantic DOM, so the current route/screen name is
  unavailable. The PNG is real visual evidence for the reviewing agent or human.
- Port 41415 is a separate development status server. Its v1 response has no process or instance
  identity, so even a successful response does not prove ownership by the selected application.

## Porting and cleanup

The tool has no package dependency and lives entirely in this directory. Copy this directory to
another worktree and pass that worktree's absolute `--workspace`, `--exe`, `--app-data-dir`, and
`--status-url`. Keep instance evidence below an ignored `.dev/review-app-inspector/` root. Cleanup is
limited to deleting that evidence root after the review; the tool never deletes it automatically.

This provisional adapter anticipates the accepted Worktree Runtime and Agent Review boundaries:
an application should eventually resolve an opaque instance handle, while development-only
adapters retain process, endpoint, window, driver, and evidence-path details. This CLI accepts raw
paths because it sits outside application composition; it must not become the product contract.

## Limits and next seam

The current release can be rendered but not semantically attached. A later review-owned launch or
application-composed attachment/evidence port is required for a truthful route, DOM/accessibility
snapshot, console/network evidence, and safe semantic controls. The accepted Agent Review branch
proved those capabilities only for specially launched isolated debug/review builds, not for this
already-running production release.
