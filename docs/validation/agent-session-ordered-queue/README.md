# Agent Session ordered queue validation

Validated on 2026-09-29 against the repository-root packaged executable.

## Automated checks

- `npm run test`: 792 frontend tests passed.
- `npm run test:rust:fast`: 790 Rust tests passed; 1 ignored.
- `npm run build`: production frontend and Tauri release build passed.
- `npm run lint`: passed with 6 existing warnings outside this slice.

## Packaged UI check

- Opened Agent Sessions in the rebuilt application through the owned WebView inspection endpoint.
- Confirmed a branch with zero worktrees remains selectable and presents `Queue worktree creation` for each eligible device.
- Confirmed the message field accepts text and enables `Send` without waiting for target preparation.
- Confirmed a migrated blocked prompt retains its text, shows the ordered preparation steps, and exposes `Retry setup`.
- Cleared the unsent validation draft after the check. No prompt was delivered.

The raw state and interaction receipts are stored in the ignored `.dev/validation/agent-session-ordered-queue` directory because they contain live session text.

## Built artifact

- Path: `codex-orchestrator.exe`
- SHA-256: `79B5324BAE786FEDE3AED3DFAE93D9F3560AC1C591BE864E95575F2A0B6BEA0F`
