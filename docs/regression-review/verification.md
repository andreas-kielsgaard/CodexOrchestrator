# Verification and replay

All checks ran against product code at `4bded63` in the new regression-review worktree on 7 September 2026. Only review documents and probes were added there.

## Executed checks

| Check                                | Result                                                      | Limit                                                                                                                               |
| ------------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `npm ci --no-audit --no-fund`        | Installed 368 packages                                      | Uses the checked-in lockfile; no dependency edits.                                                                                  |
| `npm run build`                      | Passed TypeScript and Vite build                            | Frontend build, not a native package or runtime proof. Vite reported its large-chunk warning.                                       |
| Focused frontend suites              | 52 tests passed in 11 files                                 | Isolated controls, clients, composition and the old Workflow screen.                                                                |
| `npm test`                           | 980 passed in 165 files, about 28 seconds                   | Some tests emitted React `act` warnings and Node SQLite experimental warnings. No test failed. Many use old routes or fake clients. |
| Browser probe below                  | Nine issues reproduced; zero page JavaScript errors         | Real React/CSS in headless Microsoft Edge, fake data and clients. No Tauri or provider calls.                                       |
| Additional frontend component probes | Five failures reproduced with in-memory React/JSDOM clients | See frontend report for the exact observations, including edits typed during a pending save.                                        |
| Backend review                       | Five direct call-path findings                              | No Rust suite, native package, runtime restart or live-provider workflow was run in this review.                                    |

The focused suites were run with:

```powershell
npm test -- src/features/executionConfiguration src/features/sessionEvents src/features/agentSessions/PerMessageRuntimeControls.test.tsx src/features/agentSessions/useAgentSessionController.test.tsx src/bootstrap/productApplicationComposition.test.ts src/infrastructure/workflowAuthoring src/infrastructure/executionConfiguration src/infrastructure/agentSessionProfiles src/infrastructure/sessionEvents src/features/workflows/WorkflowScreen.test.tsx
```

The additional frontend review also ran 9 tests across 4 of these files. Do not add those counts to the full-suite total: they overlap.

## Browser reproduction

Files:

- [Fixture entry](probes/browser.html) and [fake clients](probes/browser.tsx).
- [Probe runner](probes/run-browser.mjs).
- [Recorded results](evidence/browser-results.json).
- Screenshots in `evidence/` show the layout, changed run ID and blocked second message. JSON records the before/after values that a single screenshot cannot show.

From this worktree, after installing dependencies:

```powershell
node docs/regression-review/probes/run-browser.mjs
```

The runner starts Vite on `127.0.0.1:2381`, opens a separate headless Edge instance, records observations, then closes both. It does not use an existing browser profile. It uses Playwright from the bundled Codex dependencies; `REVIEW_NODE_MODULES` can point to another installed `node_modules` directory containing Playwright. Microsoft Edge must be installed.

This is an observation script, not a passing-regression test suite. `reproduced: true` means it saw the current bug. After a repair, update or promote the relevant check into the normal tests rather than treating reproduction as success.

Fixture limits:

- The fake runtime offers two models and one Capability Profile allows only the first. This tests node narrowing; it is not provider discovery.
- Save/activation clients hold data in memory. They match the source contracts but do not prove SQLite persistence.
- Compilation returns a one-item placeholder so the screen can show a result count. It does not prove compiler correctness.
- Ordinary Session creation returns the no-profile shape found in the real backend. The first invocation completes immediately in the fixture. The probe proves the mounted UI blocks the next send; source review establishes why real ordinary creation supplies that shape.
- Workflow dispatch and profile sends cannot call a provider from this fixture.

## Isolation

Prepared a separate runtime manifest with:

```powershell
npm run runtime:worktree -- prepare --instance regression-review --session session-event-regression-review --slot 48
```

Manifest: `.dev/worktree-runtime/regression-review/manifest.json` (local generated state, not committed). This prepares the native review instance; it was not started. The browser checks use their own temporary Vite server instead. The earlier `session-events-demo` instance was not modified.

## Important follow-up proof gaps

Before calling the repaired build functional, add a small fake-runtime integration check for each of these paths:

1. Ordinary creation → first completed turn → second send through the mounted client contract.
2. Stored Workflow instance → target directory → first Session launch.
3. Normal completion notification → matching connection → delivery to the next node.
4. Shared profile changes → another message to an existing pinned Session.
5. Actual backend save/activate response → protected local editor state.

Provider credentials, expanded CLI controls, full recovery/retry behavior and a future run graph are not needed for these checks. A later live-provider and restart check should be reported separately.
