# Application inspector

A development CLI for observing an explicitly identified Codex Orchestrator instance and performing bounded review interactions. It does not add product commands or orchestration authority. Run commands from the repository root; example paths and PID below must identify the intended local instance.

Start the optional status endpoint explicitly with `npm run dev:status` when an inspection needs it; the normal desktop launcher no longer starts it. Marker commands remain `npm run mark:stale` and `npm run clear:stale`.

## Observe and compare

```powershell
node review-tools/app-inspector/review-app.mjs inspect --workspace "C:\work\Codex Orchestrator" --exe "C:\work\Codex Orchestrator\src-tauri\target\debug\codex-orchestrator.exe" --pid 1234 --instance local-review --out "C:\review\before.json"
node review-tools/app-inspector/review-app.mjs compare --before "C:\review\before.json" --after "C:\review\after.json" --format human
node review-tools/app-inspector/review-app.mjs --help
```

Use `--app-data-dir` or `--database` to select explicit storage, `--evidence-root` for screenshots, and `--status-url` for the optional development endpoint. `--no-screenshot` skips native capture. Keep executable, PID, application data and evidence paths associated with the same instance.

Inspection observes processes/windows, executable/file hashes, source facts and optional status responses. SQLite is opened read-only with `query_only=ON`; its summaries exclude submitted message text, raw runtime payloads and credentials. Rows are labeled recorded evidence. Source-to-binary association inferred from path containment is weaker than a producing-commit record.

The v1 status endpoint at port 41415 has no process/instance identity. A successful response does not prove it belongs to the selected application. Its consumer is `review-app.mjs` through `src/status-adapter.mjs`, independently of the removed product widget.

## Wait for a change

`review-app.mjs wait` accepts the same instance inputs and observes a stable `visual`, `durable` or `either` change. It captures a baseline unless `--before` supplies one. `--poll-ms`, `--stable-observations` and `--timeout-ms` bound the observation; `--before-out`, `--after-out`, `--comparison-out`, `--human-out` and `--out` retain results. Use the CLI help for the complete argument list.

Exit status is 0 for the selected stable change, 2 for timeout, 130 for cancellation, or 1 for invalid input/setup failure. Visual change can be animation or caret activity; durable change is limited to summarized tables. Neither proves the intended semantic action happened. Compare the actual observations needed for that claim.

`launch-wait` starts a detached evidence watcher and returns its PID and paths. It requires `--watcher-log` and `--launch-out`; `--cancel-file` selects an explicit cancellation signal. Creating that file requests graceful completion of the evidence record. Forced process termination can prevent finalization. It does not interact with the application.

`--callback-spec` is rejected before launch. This tool has no supported desktop-task wake transport. The historical experiment with `codex exec resume` started a separate CLI turn instead of resuming the visible task; that failed approach is not part of the current operating procedure.

## Rendered WebView state and interaction

For an isolated development instance deliberately launched with a loopback WebView2 debugging port, use `webview-control.mjs`:

```powershell
node review-tools/app-inspector/webview-control.mjs snapshot --exe "C:\review\codex-orchestrator.exe" --pid 1234 --debug-url http://127.0.0.1:9231 --target-url http://127.0.0.1:1420/ --out "C:\review\state.json" --screenshot "C:\review\screen.png"
node review-tools/app-inspector/webview-control.mjs --help
```

The launch setting is `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>`. The tool checks the exact owner executable/PID, loopback listener ownership and one matching page URL before each operation. It does not attach a debug endpoint to an existing production process.

Snapshot reads rendered text, controls, selectors, geometry, focus, scroll regions and accessibility state without input or scrolling. PNG capture reads the WebView compositor, excluding native chrome/dialogs. Password values are redacted; other visible content is included. Reads occur sequentially and can be truncated; they do not establish durable or provider outcomes.

The same owner options support `click --selector`, `type --selector --text-file`, and `select --selector --value`, each with an output receipt. They resolve one bounded control and send CDP Input events; arbitrary scripts and coordinates are not accepted. Selection traverses enabled options using Home/ArrowDown and may emit intermediate changes. A subsequent snapshot verifies the resulting selection. Receipts redact entered text and distinguish ownership checks, dispatched input and unobserved semantics.

`interact-app.mjs click --exe <absolute-exe> --pid <pid> --x <x> --y <y> --out <receipt>` is the separate native Windows mouse-message transport. Coordinates must be 0–32767 and inside the named process window's client area; the HWND must belong to that process or a live descendant. It does not foreground the window or use the clipboard. Acknowledged mouse messages still require a separate product-state observation.

These process/endpoint checks are point-in-time observations, not race-free identity proof. Debug endpoints and raw evidence paths belong to development tooling rather than the product's application contract.

## Checks

The inspector uses Node's test runner, not Vitest. It has no additional package dependency for ordinary checks. Run the eight non-browser test files explicitly:

```powershell
node --test review-tools/app-inspector/test/interaction-adapter-framing.test.mjs review-tools/app-inspector/test/launch-paths.test.mjs review-tools/app-inspector/test/rendered-state.test.mjs review-tools/app-inspector/test/snapshot-compare.test.mjs review-tools/app-inspector/test/wait-for-change.test.mjs review-tools/app-inspector/test/webview-control.test.mjs review-tools/app-inspector/test/windows-adapter-framing.test.mjs review-tools/app-inspector/test/windows-webview-owner-boundary.test.mjs
```

The separate `node --test review-tools/app-inspector/test/webview-control-live.test.mjs` launches an installed Chromium browser. Windows adapter checks exercise their own local boundaries. Neither command is a live Codex or full product acceptance test. The future tooling task may add named npm wrappers; at `60c3798` they are not present.

## Source and evidence

The directory owns the CLI, Windows/HTTP/WebView adapters, comparison and wait logic. Evidence files under an explicitly chosen ignored `.dev/` directory are review outputs; the inspector does not delete them automatically. Reusing the tool in another worktree requires that worktree's actual instance and storage inputs.

Historical operating guidance is retrievable at `e2bfc6c:review-tools/app-inspector/README.md`. The original inspector-design request was not recovered during consolidation; current behavior above was checked against the CLI help and source. Past task-specific paths and Review Coach procedures do not define current product behavior. See [development](../../docs/development.md) and [validation evidence](../../docs/validation-evidence.md) for the wider proof boundaries.
