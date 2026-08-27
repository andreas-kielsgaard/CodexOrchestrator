use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tauri::State;

const READ_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

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
    pub(crate) git_common_directory: String,
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
struct RegisteredRepository {
    id: String,
    name: String,
    root_path: String,
}

#[derive(Clone, Debug)]
struct RegisteredBranch {
    id: String,
    repository_id: String,
    name: String,
}

#[derive(Clone, Debug)]
struct RegisteredWorktree {
    id: String,
    repository_id: String,
    branch_name: Option<String>,
    path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CurrentBranchWorktree {
    path: String,
    branch_name: String,
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
        let repositories = query_registered_repositories(&connection)?;
        let branches = query_registered_branches(&connection)?;
        let worktrees = query_registered_worktrees(&connection)?;
        drop(connection);
        resolve_current_targets(
            repositories,
            branches,
            worktrees,
            discover_current_branch_worktrees,
            discover_git_common_directory,
        )
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

fn query_registered_repositories(
    connection: &Connection,
) -> Result<Vec<RegisteredRepository>, String> {
    query_rows(
        connection,
        "SELECT id,name,root_path FROM repos ORDER BY id",
        "repositories",
        |row| {
            Ok(RegisteredRepository {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
            })
        },
    )
}

fn query_registered_branches(connection: &Connection) -> Result<Vec<RegisteredBranch>, String> {
    query_rows(
        connection,
        "SELECT id,repo_id,name FROM branches ORDER BY id",
        "branches",
        |row| {
            Ok(RegisteredBranch {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                name: row.get(2)?,
            })
        },
    )
}

fn query_registered_worktrees(connection: &Connection) -> Result<Vec<RegisteredWorktree>, String> {
    query_rows(
        connection,
        "SELECT worktree.id,worktree.repo_id,branch.name,worktree.path
         FROM worktrees worktree
         LEFT JOIN branches branch
           ON branch.id=worktree.branch_id AND branch.repo_id=worktree.repo_id
         ORDER BY worktree.id",
        "worktrees",
        |row| {
            Ok(RegisteredWorktree {
                id: row.get(0)?,
                repository_id: row.get(1)?,
                branch_name: row.get(2)?,
                path: row.get(3)?,
            })
        },
    )
}

fn query_rows<T>(
    connection: &Connection,
    sql: &str,
    label: &str,
    map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("Unable to prepare discovered {label} query: {error}"))?;
    let rows = statement
        .query_map([], map)
        .map_err(|error| format!("Unable to query discovered {label}: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to read discovered {label}: {error}"))
}

fn resolve_current_targets(
    repositories: Vec<RegisteredRepository>,
    branches: Vec<RegisteredBranch>,
    worktrees: Vec<RegisteredWorktree>,
    mut discover_worktrees: impl FnMut(&str) -> Result<Vec<CurrentBranchWorktree>, String>,
    mut discover_identity: impl FnMut(&str) -> Result<String, String>,
) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
    let branch_by_name = branches
        .into_iter()
        .map(|branch| {
            (
                (branch.repository_id.clone(), branch.name.clone()),
                branch.id,
            )
        })
        .collect::<HashMap<_, _>>();
    let mut targets = Vec::new();

    for repository in repositories {
        let current = discover_worktrees(&repository.root_path).map_err(|error| {
            format!(
                "Unable to discover current worktrees for {}: {error}",
                repository.root_path
            )
        })?;
        let git_common_directory = discover_identity(&repository.root_path).map_err(|error| {
            format!(
                "Unable to resolve Git identity for {}: {error}",
                repository.root_path
            )
        })?;
        for worktree in current {
            let branch_id = branch_by_name
                .get(&(repository.id.clone(), worktree.branch_name.clone()))
                .cloned()
                .unwrap_or_else(|| {
                    temporary_id(
                        "branch",
                        &format!("{}\0{}", repository.id, worktree.branch_name),
                    )
                });
            let worktree_id = worktrees
                .iter()
                .find(|registered| {
                    registered.repository_id == repository.id
                        && registered.branch_name.as_deref() == Some(&worktree.branch_name)
                        && crate::same_filesystem_path(&registered.path, &worktree.path)
                })
                .map(|registered| registered.id.clone())
                .unwrap_or_else(|| temporary_id("worktree", &worktree.path));
            targets.push(ResolvedRepoBranchWorktreeTarget {
                repository: ResolvedRepositoryTarget {
                    id: repository.id.clone(),
                    name: repository.name.clone(),
                    git_common_directory: git_common_directory.clone(),
                },
                branch: ResolvedBranchTarget {
                    id: branch_id,
                    name: worktree.branch_name,
                },
                worktree: ResolvedWorktreeTarget {
                    id: worktree_id,
                    path: worktree.path,
                },
            });
        }
    }

    targets.sort_by(|left, right| {
        target_sort_key(left)
            .cmp(&target_sort_key(right))
            .then_with(|| left.worktree.id.cmp(&right.worktree.id))
    });
    Ok(targets)
}

fn discover_current_branch_worktrees(
    repository_root: &str,
) -> Result<Vec<CurrentBranchWorktree>, String> {
    Ok(crate::git_worktree_facts(repository_root)?
        .into_iter()
        .filter(|worktree| Path::new(&worktree.path).is_dir())
        .filter_map(|worktree| {
            worktree
                .branch_name
                .map(|branch_name| CurrentBranchWorktree {
                    path: worktree.path,
                    branch_name,
                })
        })
        .collect())
}

fn discover_git_common_directory(repository_root: &str) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|error| format!("Unable to run git rev-parse: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|_| "Git common directory was not valid UTF-8".to_string())?;
    let path = path.trim();
    if path.is_empty() {
        return Err("Git returned an empty common directory".into());
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("Unable to canonicalize Git common directory: {error}"))?;
    Ok(canonical.to_string_lossy().into_owned())
}

fn temporary_id(kind: &str, value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("temporary-{kind}-{hash:016x}")
}

fn target_sort_key(target: &ResolvedRepoBranchWorktreeTarget) -> (String, String, String) {
    (
        target.repository.name.to_lowercase(),
        target.branch.name.to_lowercase(),
        crate::normalize_path_for_compare(&target.worktree.path),
    )
}

#[cfg(test)]
mod tests;
