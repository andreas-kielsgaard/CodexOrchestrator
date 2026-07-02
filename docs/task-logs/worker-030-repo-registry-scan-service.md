# Worker 030 - Repo Registry / Scan Service Boundary

Date: 2026-07-02

## Summary

Added a focused application service for registering/scanning a repository through injected
boundaries. `registerAndScanRepo` accepts a project ID, root path, optional default branch, and
optional scan timestamp; calls an injected `GitRepoScanner`; maps the scan to domain facts; persists
repo/branch/worktree records through `RepoSyncStore`; and returns the synced records with compact
scan and sync metadata for future UI/runtime wiring.

## Files Changed

- `src/application/repoRegistryScan.ts`
- `src/application/repoRegistryScan.test.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-030-repo-registry-scan-service.md`

## Verification

- `npm run test -- src/application/repoRegistryScan.test.ts` - passed
- `git diff --check main...worker/030-repo-registry-scan-service` - passed
- `npm run lint` - passed
- `npm run format:check` - passed
- `npm run test` - passed, 35 files / 219 tests
- `npm run build` - passed
- `cargo --version` / `rustc --version` - unavailable on `PATH`; `npm run build:tauri` not run.

## Blockers

- `npm run build:tauri` remains blocked because Rust/Cargo are not available on `PATH`.

## Review Notes

- No SQLite store contract was extended. The existing `RepoSyncStore` already supports the
  scan-and-persist path needed for this slice; list/remove behavior is deferred until a caller needs
  a concrete registry management surface.
- The service intentionally does not create worktrees, link tasks to worktrees, wire React UI, open
  database files, or execute Git directly.
- The service returns summarized scan metadata and applied change counts rather than raw command
  output as the primary API.
