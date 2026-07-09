# Prototype Database Reset and Upgrade Procedure

Status: AS-00 development procedure

The Agent Session reset does not imply that a developer's local SQLite database was reset. A
database created by the archived prototype may contain these durable migration-ledger entries:

| ID                                    | Position | Policy   |
| ------------------------------------- | -------- | -------- |
| `006_orchestration_drafts_schema`     | 5        | reserved |
| `007_orchestration_stage_runs_schema` | 6        | reserved |
| `008_agent_sessions_schema`           | 7        | reserved |

Those IDs and positions must never be reused. Current schema migrations retain their original
explicit positions `0` through `4`; the next forward migration must begin with a new ID and
position, no earlier than `009` at position `8`.

## Safety Rules

- Exit Codex Orchestrator before copying, moving, or inspecting its database files. Confirm that
  no app process remains. This keeps the database, write-ahead log, and shared-memory sidecar in a
  consistent set.
- Preserve the whole app-data directory when making a prototype backup. Do not copy only the main
  `.sqlite` file while the app is running.
- Do not delete prototype data, drop prototype tables, edit `schema_migrations`, or renumber ledger
  rows by hand.
- Test migration behavior against a copy or an in-memory fixture. Never use the real app database
  as a migration test target.

## Locate and Audit the Windows Development Database

The Tauri identifier is `dev.codex-orchestrator.app`, so the normal Windows development path is:

```text
%APPDATA%\dev.codex-orchestrator.app\codex-orchestrator.sqlite
```

With the app stopped, inspect only the ledger and table names through a read-only SQLite
connection:

```sql
SELECT id, applied_at, position
FROM schema_migrations
ORDER BY position;

SELECT name
FROM sqlite_master
WHERE type = 'table'
ORDER BY name;
```

If the archived IDs are absent, follow the normal migration path. If any archived ID is present,
choose exactly one of the following procedures.

## Preferred Procedure: Non-Destructive Development Reset

The archived Agent Session and orchestration schemas were never shipped. Unless prototype records
must be retained, move the complete app-data directory aside and let the app create a clean one.
The following PowerShell commands do not delete the prototype directory:

```powershell
$appData = Join-Path $env:APPDATA 'dev.codex-orchestrator.app'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$backup = "$appData.prototype-backup-$stamp"

Move-Item -LiteralPath $appData -Destination $backup
Write-Output "Prototype database preserved at $backup"
```

Then start Codex Orchestrator once. A new install follows one schema path and records only current
migrations `001` through `005` at positions `0` through `4`. Re-run the read-only audit against the
new database. Keep the renamed prototype directory until its data is explicitly declared
unneeded; this procedure does not include a deletion step.

If the source directory does not exist, do not create a backup placeholder. Starting the app will
create the normal directory and database.

## Retained-Data Procedure: Forward Upgrade Only

If prototype data matters, do not reset it and do not alter its ledger. With the app stopped, first
copy the complete app-data directory to a timestamped backup location. The AS-00 coordinator can
initialize current task storage while leaving archived positions `5` through `7` and their tables
untouched.

Retaining the database is not the same as adopting the archived Agent Session schema. Before new
Agent Session persistence is enabled, a reviewed forward migration must:

1. use a new immutable ID and position, beginning no earlier than `009` at position `8`;
2. detect which archived tables and columns actually exist rather than trusting the ledger alone;
3. transform or quarantine prototype records transactionally;
4. leave the archived ledger rows unchanged; and
5. prove both clean-install and retained-prototype paths with disposable database fixtures.

Until that forward migration exists, archived Agent Session records are unsupported prototype
data. Keep the backup and avoid any manual partial upgrade.

## Recovery

If a reset build must be rolled back, exit the app, move the newly created app-data directory to a
separate diagnostic name, and move the preserved prototype directory back to the original
`dev.codex-orchestrator.app` path. Move directories only while the app is stopped so the database
and any sidecars remain together.
