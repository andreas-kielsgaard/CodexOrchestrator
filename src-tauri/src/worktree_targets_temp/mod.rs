use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
use tauri::State;

const READ_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const DISCOVERED_TARGETS_QUERY: &str = r#"
SELECT
    repos.id,
    repos.name,
    repos.root_path,
    branches.id,
    branches.name,
    worktrees.id,
    worktrees.path
FROM worktrees
JOIN repos ON repos.id = worktrees.repo_id
JOIN branches
    ON branches.id = worktrees.branch_id
    AND branches.repo_id = worktrees.repo_id
ORDER BY
    repos.name COLLATE NOCASE,
    repos.name,
    repos.root_path COLLATE NOCASE,
    repos.root_path,
    repos.id,
    branches.name COLLATE NOCASE,
    branches.name,
    branches.id,
    worktrees.path COLLATE NOCASE,
    worktrees.path,
    worktrees.id
"#;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRepoBranchWorktreeTarget {
    pub(crate) repository: ResolvedRepositoryTarget,
    pub(crate) branch: ResolvedBranchTarget,
    pub(crate) worktree: ResolvedWorktreeTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRepositoryTarget {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) root_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedBranchTarget {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedWorktreeTarget {
    pub(crate) id: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DiscoveredWorktreeTargetSource {
    database_path: PathBuf,
}

impl DiscoveredWorktreeTargetSource {
    pub(crate) fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub(crate) fn list(&self) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
        let connection = self.open_read_only_connection()?;
        query_discovered_worktree_targets(&connection)
    }

    fn open_read_only_connection(&self) -> Result<Connection, String> {
        let connection =
            Connection::open_with_flags(&self.database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|error| {
                    format!("Unable to open discovered worktree storage read-only: {error}")
                })?;
        connection
            .busy_timeout(READ_BUSY_TIMEOUT)
            .map_err(|error| format!("Unable to configure discovered worktree storage: {error}"))?;
        Ok(connection)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WorktreeTargetsTempState {
    source: DiscoveredWorktreeTargetSource,
}

impl WorktreeTargetsTempState {
    pub(crate) fn new(database_path: PathBuf) -> Self {
        Self {
            source: DiscoveredWorktreeTargetSource::new(database_path),
        }
    }
}

#[tauri::command]
pub(crate) fn list_discovered_worktree_targets(
    state: State<'_, WorktreeTargetsTempState>,
) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
    state.source.list()
}

fn query_discovered_worktree_targets(
    connection: &Connection,
) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
    let mut statement = connection
        .prepare(DISCOVERED_TARGETS_QUERY)
        .map_err(|error| format!("Unable to prepare discovered worktree target query: {error}"))?;
    let targets = statement
        .query_map([], |row| {
            Ok(ResolvedRepoBranchWorktreeTarget {
                repository: ResolvedRepositoryTarget {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_path: row.get(2)?,
                },
                branch: ResolvedBranchTarget {
                    id: row.get(3)?,
                    name: row.get(4)?,
                },
                worktree: ResolvedWorktreeTarget {
                    id: row.get(5)?,
                    path: row.get(6)?,
                },
            })
        })
        .map_err(|error| format!("Unable to query discovered worktree targets: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to read discovered worktree targets: {error}"))?;
    Ok(targets)
}

#[cfg(test)]
mod tests;
