# Mutable session target validation

Implementation worktree: `feature/Remote-Development`, rebased onto main `a806212`.

## Native continuation

The isolated Windows → Linux → Windows test passed with Codex `0.154.0` on both devices. Native thread `01a0a1cf-32fb-7a01-9446-1290441526ac` retained its identity through a same-device working-directory change, a remote turn, and a return turn. The final response recalled separate tokens introduced on the laptop and server. Readiness reported the destination cwd before each prompt was released.

Evidence: `.dev/continuation-proof.log`, `.dev/continuation-latest.json`, and the referenced isolated run's `evidence.json`. These are engine/host integration results; they do not by themselves establish desktop UI acceptance. The experiment used a new conversation and separate native homes on each device. Authentication remained on its owning device.

The server's `/root/.local/bin/codex` now points to the official `0.154.0` binary. Its previous `0.144.0` installation remains available. The configured Orchid host was rebuilt and deployed using `scripts/deploy-orchid-host.ps1`.

## Destination worktree

An isolated published Git fixture on the server passed read-only branch-tip discovery, exact-commit materialization, and reuse of the same instance on retry. Discovery did not create a worktree. Created commit: `0747e22fb0303685b232ae7ead57bf171e2471ab`.

Evidence: `.dev/remote-worktree-preparation-proof.json`. Engine tests also cover a published branch advancing after confirmation while materialization retains the previously confirmed commit, and ignores unpublished source changes.

## Desktop integration

Verified on 2026-09-16:

- Frontend: 125 tests across 28 files; TypeScript, lint, changed-file formatting, and UTF-8 checks passed.
- Engine: 41 tests passed.
- Desktop: 6 preparation, 1 migration, 5 remote-runtime, 3 checkout, and 6 import tests passed. One live import test remains ignored.
- Final debug application build passed. Compiler warnings remain.

Retained executable: `C:\Users\user\AppData\Local\CodexOrchestrator\builds\322e8e9d195da2c4\b68c3bc1-e542-43e1-9eb1-b7f54e31cd72\output\codex-orchestrator.exe`.

Use `CODEX_ORCHESTRATOR_APP_DATA_DIR=C:\Users\user\.codex-orchestrator\remote-development-laptop` to access the configured remote-development demo data. A pre-upgrade database backup is retained at `.dev/session-preparation-before-upgrade.sqlite`.

Final native UI acceptance remains outstanding: automatic approval review rejected launching the retained executable with “blocked by policy,” without a more specific reason. Earlier screenshots used an older backend and are not acceptance evidence for this build. In particular, the final toolbar, creation confirmation, writable draft during preparation, and desktop device-switching flow have automated coverage but have not been verified together in this final native build.

Detailed local logs: `.dev/session-preparation-final-build.log` and `.dev/check-*.log`. The disposable published repository fixture for UI verification is described in `.dev/session-target-demo/fixture.json`; it has not been registered in the application.
