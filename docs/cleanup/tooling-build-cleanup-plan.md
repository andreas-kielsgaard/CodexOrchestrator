# Tooling and build cleanup

Status: implemented and validated on `cleanup/tooling-build-residue`; not merged or pushed.
Baseline inspected: `e2bfc6c`, 2026-09-14.
Implementation evidence: [validation record](./tooling-build-cleanup-validation.md).

## Target and decisions

Make installation, build output, and validation scope explicit. Retire the obsolete standalone
Agent Session scenario page and its simulator. Preserve useful application assertions through
the current client contracts.

- npm is the supported package manager; preserve the existing resolved dependency versions.
- The recorded Session page is obsolete. Remove it without a replacement harness build.
- Reuse of its simulator by tests does not justify retaining or relocating the simulator.
- Preserve other developer previews through fixed Session records; remove their dependency on
  the obsolete simulator and make simulated Session mutations explicitly unsupported.
- Keep the named fast/full Rust test profiles and optional PowerShell helpers.
- Keep ordinary checks separate from installed-browser, installed-Codex, and paid-provider checks.

This scope excludes product Harness configuration, the retired task implementation, retirement
of other review pages, general documentation archival, cache deletion, worktree cleanup, and dependency upgrades.
Preserve unrelated changes to the repository-session navigation and remote-worktree documents.

The final dependency check corrected an earlier omission: the recorded development composition
also imports the simulator through `../agentSessions`. This serves the Plan Builder, Harness
Inspector, and Work Unit review previews, plus the file-review preview indirectly. The plan
assumes their recorded histories should remain available for browsing and inspection, while
their simulated Session creation, sending, and cancellation end. This is a developer-preview
behavior change; the real application client is unchanged. Retiring those other previews would
require a separate scope decision.

## Ownership after cleanup

| Responsibility                      | Owner and consumers                                                                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Dependencies and supported commands | `package.json` and `package-lock.json`; README, launcher, Tauri, and Worktree Review agree on npm.                                    |
| Frontend application output         | `vite.config.ts`, using the standard `index.html` entry; retain runtime-root and output-directory controls.                           |
| Ordinary desktop build              | Tauri's existing `beforeBuildCommand` invokes the frontend build once.                                                                |
| Worktree Review build               | Existing Rust build runner explicitly typechecks, builds the frontend, and invokes Tauri with its hook disabled and isolated outputs. |
| Session sample records              | Existing `src/features/agentSessions/testFixtures.ts` and `profileTestFixtures.ts`; plain current DTOs and existing profile setup.    |
| Test-specific responses and events  | Local stubs/spies in each consuming test file; tests choose returned records and when a response or event arrives.                    |
| Retained preview Session reads      | `src/dev/orchestrationSection/recordedOrchestrationClient.ts`; a local fixed-record adapter using the preview's existing DTOs.        |
| Optional developer tools            | Existing scripts and their documentation; no new shared runner or simulator.                                                          |

## Origin of the package-manager ambiguity

- `07f2c63` (2026-07-01) established npm setup and `package-lock.json`. The React build plugin
  was already under runtime dependencies in that bootstrap commit.
- Both pnpm files were untracked in worktree `8bb8` before the 2026-08-02 baseline snapshot.
  Session `019fc106-1222-7f52-a1ad-9189481658e8` records that status at line 443 and the isolated-index
  `git add -A -- .` / commit sequence at lines 693-702.
- `fe40951` captured the existing worktree contents in a 118-file baseline commit. It added the
  pnpm files without changing npm setup, its lockfile, or the Tauri build configuration.
- The pnpm files have not changed on the current lineage since that commit; the workspace file
  already contained `esbuild: set this to true or false` when added.

This establishes how the competing files entered the baseline. The original command or actor
that created them remains unidentified. The inspected evidence establishes preservation of
existing material, not a documented decision to support a second package manager.

## 1. Dependency configuration and command surface

Change `package.json`:

- Change `install:app` to `npm ci --include=dev`, matching launcher recovery.
- Move `@vitejs/plugin-react` from dependencies to devDependencies, retaining its version range.
- Rename `validate:product` to `validate:worktree-review`, keeping its existing command sequence.
- Replace `validate:release` with `validate:release-build`: `tauri build --no-bundle`.
  Tauri owns its frontend prerequisite and compiles the native release executable. Keep
  `check:rust:release` available as an independent compile check.
- Keep `build`, `build:tauri`, the frontend and Rust test commands, and the explicit Codex
  contract-test command. Do not keep aliases for the two misleading validation names; no tracked
  callers were found outside the manifest.
- Add `test:app-inspector` invoking Node's test runner with the eight current non-browser test
  files explicitly listed below. Add `test:app-inspector:browser` for the separate browser test.

`test:app-inspector` selects files under `review-tools/app-inspector/test/`:

- `interaction-adapter-framing.test.mjs`
- `launch-paths.test.mjs`
- `rendered-state.test.mjs`
- `snapshot-compare.test.mjs`
- `wait-for-change.test.mjs`
- `webview-control.test.mjs`
- `windows-adapter-framing.test.mjs`
- `windows-webview-owner-boundary.test.mjs`

The browser command selects only `webview-control-live.test.mjs`, which launches an installed
Chromium browser. Keep the existing Vitest exclusion for these Node tests. Do not add either
command to ordinary frontend or Worktree Review validation.

Delete `pnpm-lock.yaml` and `pnpm-workspace.yaml`. Update npm lockfile dependency classification
without changing resolved versions or integrity values. Verify the resulting manifest and lockfile
with a clean npm install in an isolated validation directory, preserving the working checkout's
installed dependencies.

Update README setup and command descriptions. Explain that release-build validation produces an
executable without validating an installer or live application behavior. Document the optional
Node, PowerShell, and installed-Codex checks and their prerequisites. A skipped executable test
does not establish executable compatibility.

## 2. Remove the obsolete page and simulator

Delete:

- `agent-session-harness.html`
- `src/dev/agentSessions/AgentSessionHarness.tsx`
- `src/dev/agentSessions/main.tsx`
- `src/dev/agentSessions/harness.css`
- `src/dev/agentSessions/index.ts`
- `src/dev/agentSessions/recordedAgentSessionClient.ts`
- `src/dev/agentSessions/recordedAgentSessionClient.test.ts`
- `src/dev/agentSessions/scenarios.ts`
- `src/features/agentSessions/AgentSessionScreen.harness.test.tsx`

Remove the multi-page `rollupOptions.input` declaration from `vite.config.ts`. Retain Vite's
existing runtime-root/cache/output settings, dev-server port controls, and test setup. Default
build output must contain the application entry and no `agent-session-harness.html`.

Keep `src-tauri/tauri.conf.json`'s build hook and `frontendDist`. Keep the distinct output handling
in `src-tauri/src/worktree_application/build.rs`: it already disables the hook after explicitly
building the frontend. Consolidating that runner into ordinary npm commands would lose its
bounded output and process responsibilities.

Adapt `src/dev/orchestrationSection/recordedOrchestrationClient.ts` before deletion. Replace its
recorded-client factory/store imports with a small `AgentSessionClient` implementation in the same
file. Preserve its existing export for Session navigation tests:

- List, load, and reload the existing `recordedAgentSessionDetails` and Harness Inspector record.
  Unknown Session IDs fail explicitly.
- Subscriptions return a no-op unsubscribe; no generated events or background progression.
- Creation, sending, and cancellation reject with a clear recorded-preview unsupported message,
  surfaced through existing application error handling. Do not report fabricated success.
- No mutable Session store, generated invocation IDs, scenario stepping, or snapshot mechanism.

Keep the existing development composition factory and callers. Its Harness metadata source,
orchestration records, proposal records, and file-review records retain their existing ownership.
No new shared client module or production capability mechanism is needed for this adaptation.

## 3. Keep product checks and remove simulation machinery

The five recorded-client tests disappear with the simulator: they test its own Map storage,
stepping, snapshot copying, and injected errors.

Existing `AgentSessionTranscript.test.tsx`, `transcriptProjector.test.ts`, and
`AgentMarkdown.test.tsx` cover most rendering assertions from the nine old screen scenarios.
Retain those focused suites. Drop the old simulated-restart and long-content cases: shared memory
across a remount is not persistence verification, and text presence does not establish layout.

Create `src/features/agentSessions/AgentSessionScreen.test.tsx` for the remaining screen behavior:

1. A running Session receives a correlated update, reloads returned history, and shows its final
   response with completed processing collapsed; expanding processing reveals the existing work.
2. An update for another Session cannot replace the selected conversation; a selected-Session
   update is reflected in that conversation.
3. An injected collection-load failure is displayed and can be dismissed. Do not preserve the old
   assertion that two identical subscription-error alerts must appear.

Use the existing current Session/profile fixtures, override only needed client methods, and
capture the update listener directly. The test supplies the next DTO; the stub does not implement
session creation, persistence, invocation transitions, or a scenario interpreter.

Adapt the other direct test consumers:

| File                                                                      | Assertions and implementation shape                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/app/App.orchestrations.test.tsx`                                     | Preserve first-send draft binding, failed-send exclusion, proposal/request routing, and title propagation. Reuse its existing local `agentClient()` base. Override methods with explicit responses/spies. Replace `.store` inspections with application request assertions and visible outcomes; assert returned Session IDs are propagated. |
| `src/features/orchestrations/OrchestrationSection.test.tsx`               | Replace the recorded client inside its local composition helper with reads of the supplied Session records. For the send interaction, assert the selected Session/message request and supply the post-send record explicitly. Preserve selection, focus, navigation, and writability checks.                                                 |
| `src/features/orchestrations/components/SharedAgentSessionPanel.test.tsx` | Preserve explicit writability, read-only inspection, no fabricated fallback on load failure, and stale-response exclusion. Use feature DTO fixtures and local load/rejection stubs. Retain the existing deferred-response test pattern.                                                                                                      |

No new shared test-support directory, relocated simulator, or general-purpose fake backend.
Small duplicated interface boilerplate is acceptable where it keeps the tested contract obvious.
Retain the other orchestration review fixtures. Their composition uses the fixed-record adapter
described above; tests requiring a write response supply that response locally instead.

## 4. Documentation and retained tooling

- `README.md`: npm setup, command scope, and links to optional tool checks.
- `docs/agent-session/README.md`: replace current instructions requiring the scenario page with
  the current Session component/controller/profile test entry points. Explain that remaining
  recorded previews support Session inspection, with Session mutations unsupported.
- `docs/orchestration/rust-test-developer-validation.md`: preserve fast/full and opt-in helper
  distinctions; adjust only affected command references.
- `review-tools/app-inspector/README.md`: document the separate ordinary and browser test commands.
- Keep historical execution-ledger, handoff, and convergence-evidence records as dated evidence.
  Their references describe earlier builds; they are not current build requirements.
- Retain `scripts/cargo-test-fast.ps1`, `scripts/cargo-sccache.ps1`, their tests, the runtime-status
  tools, and `launch-dev.bat`. This plan does not retire those separate workflows.

Validation exposed two additional changes within the tooling/test scope:

- `eslint.config.js`: declare Node globals for the existing review scripts under documentation
  and browser globals for the two `run-browser.mjs` probes. Keep those scripts linted.
- `EpicPlanBuilder.test.tsx`: wait for the Plan action to become enabled before clicking; its
  presence alone does not mean asynchronous Session loading has completed.

## Order and verification

1. Adapt the retained preview composition and useful test consumers; add the focused screen
   checks. Remove redundant tests and the recorded simulator/page. Remove the extra Vite entry.
2. Normalize npm configuration and build/validation commands. Update the relevant instructions.
3. Review the complete diff and verify the result:
   - Resolve relative imports and re-exports, including `../agentSessions`, to verify no source
     or test depends on the retired directory. Confirm no active harness build entry remains.
   - Resolved npm dependency versions/integrities unchanged; clean `npm ci --include=dev` succeeds.
   - Focused changed suites, then the full frontend suite, pass. Fix act warnings introduced or
     exposed in the rewritten cases; distinguish unrelated existing warnings.
   - Include indirect composition consumers: `App.agentSessions`, `App.harnessInspector`,
     `App.sessionEventModel`, `AgentSessionScreen.navigation`, `recordedOrchestrationClient`,
     `recordedEpicProductDecisionSource`, and `recordedWorkUnitReview.consumer` tests.
   - Open the retained Plan Builder, Harness Inspector, Work Unit review, and file-review previews.
     Verify recorded Session history and inspection still load; a Session mutation produces clear
     unsupported feedback and does not alter records. Preserve existing Harness metadata preview
     behavior. Do not add simulator self-tests to replace the deleted ones.
   - `npm run test:app-inspector` executes the intended eight files without launching its browser
     test. The optional browser and Codex tests remain separate.
   - ESLint, formatting of changed files, and `git diff --check` pass.
   - A clean frontend build contains only the application HTML entry. Verify isolated output
     overrides still work; stale files in an old `dist` are not evidence of current build contents.
   - Run `validate:worktree-review` and `validate:release-build`. Confirm the release path runs the
     frontend build once and produces the expected native executable.
   - Launch that executable with disposable application and WebView2 data directories and verify
     the normal application opens. Do not invoke a provider. Installer validation is outside this
     executable-build check.

Completion means the simulator and redundant coverage are gone, valuable application assertions
still pass, and the documented npm/build/check commands match their actual behavior. It does not
require preserving the former test count.
