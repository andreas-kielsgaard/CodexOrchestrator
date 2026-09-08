# Managed database demonstration

These scenarios demonstrate the user-visible outcomes of the managed active database. They use the
same Workflow, Agent Session, Native Profile, and Harness implementations as the desktop.

## Prerequisites

- Build this branch's application binary.
- Select a ready Native Profile in Technical Settings.
- Use the desktop AppData directory containing both `codex-orchestrator-active-v3.sqlite` and the
  legacy `codex-orchestrator.sqlite` worktree catalog.
- Select a currently discovered worktree. The demonstration creates
  `handoffs/managed-database-demo.md` there.

Set explicit PowerShell values:

```powershell
$root = "C:\absolute\database-write-hardening\Codex Orchestrator"
$binary = "$root\src-tauri\target\debug\codex-orchestrator.exe"
$appData = "C:\absolute\desktop-app-data"
$worktree = "C:\absolute\discovered-worktree"
$catalog = "$root\examples\database-hardening\managed-database-parallel-fanout.json"
```

Build and launch the desktop from this branch. Stop any other Orchestrator build that uses the same
AppData before launching this one; then leave this desktop open during the scenarios.

```powershell
cargo build --manifest-path "$root\src-tauri\Cargo.toml" --no-default-features
$env:CODEX_ORCHESTRATOR_APP_DATA_DIR = $appData
& $binary
```

Import the definition once. The desktop may remain open:

```powershell
& $binary workflow-cli import --app-data-dir $appData --file $catalog
```

## Scenario 1: parallel Workflow fan-out

Create an instance in the desktop using **Demo · Managed Database Parallel Fan-out**, select the
**Plan author**, and request a small implementation plan.

Expected evidence:

- the author creates `handoffs/managed-database-demo.md`;
- one completion records three successful connection activations;
- Runtime reviewer, Boundary reviewer, and Test reviewer run in separate fresh Sessions;
- the instance finishes with four associated Sessions and no database-lock error.

Send a second message to Plan author to revise the plan. Three additional fresh receiver Sessions
should be created while the original Sessions remain inspectable.

## Scenario 2: desktop and concurrent CLI writers

Keep the desktop open on the Workflow overview, then run with PowerShell 7:

```powershell
& "$root\examples\database-hardening\run-concurrent-cli.ps1" `
  -ApplicationBinary $binary `
  -AppDataDir $appData `
  -Worktree $worktree
```

The runner starts one Workflow send and four instance creations in separate processes against the
same active database. It writes JSON evidence under `%TEMP%` and fails if any process reports
`database is locked`, a connection activation fails, fewer than four Sessions are created, or an
invocation does not complete. The new instances should also appear in the already-running desktop.

## Scenario 3: transaction-safety proof

This fast scenario does not launch providers:

```powershell
cargo test --manifest-path "$root\src-tauri\Cargo.toml" --profile test-fast `
  persistence::active_database::tests --lib -- --test-threads=1

cargo test --manifest-path "$root\src-tauri\Cargo.toml" --profile test-fast `
  agent_sessions::repository::tests --lib -- --test-threads=1

cargo test --manifest-path "$root\src-tauri\Cargo.toml" --profile test-fast `
  different_receiver_fan_out_creates_and_launches_both_invocations --lib -- --test-threads=1
```

Expected result: 12 tests pass. They demonstrate serialized same-process writes, coordination
between independent database handles, query-only readers, rollback after a rejected operation,
nested-operation rejection without deadlock, and concurrent pending Agent Session invocations
without SQLite lock-upgrade failure. The final test proves that one completed sender can persist and
launch multiple receiver invocations.
