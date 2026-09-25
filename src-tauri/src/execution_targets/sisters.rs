//! Product-owned sibling-worktree identity and selection locks.
//!
//! This deliberately records only Orchid's current editing authority. Git remains the source of
//! physical state; the record lets target selectors avoid dispatching a second Session to a sister
//! that Orchid has already moved elsewhere.

use super::domain::SessionExecutionTarget;
use crate::persistence::ActiveDatabase;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::sync::Arc;

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sister_worktree_groups (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    branch_ref TEXT NOT NULL,
    active_device_id TEXT NOT NULL,
    owner_session_id TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(repository_id, branch_ref)
);
CREATE TABLE IF NOT EXISTS sister_worktree_instances (
    sister_group_id TEXT NOT NULL REFERENCES sister_worktree_groups(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    path TEXT NOT NULL,
    head TEXT,
    branch_ref TEXT NOT NULL,
    PRIMARY KEY(sister_group_id, device_id, worktree_id)
);
"#;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SisterWorktreeLock {
    pub(crate) sister_group_id: String,
    pub(crate) active_device_id: String,
    pub(crate) owner_session_id: String,
}

pub(crate) struct SisterWorktreeStore(Arc<ActiveDatabase>);

impl SisterWorktreeStore {
    pub(crate) fn new(database: Arc<ActiveDatabase>) -> Self {
        Self(database)
    }

    pub(crate) fn lock_for(
        &self,
        repository_id: &str,
        branch_ref: &str,
    ) -> Result<Option<SisterWorktreeLock>, String> {
        self.0
            .read("read sister worktree lock", |connection| {
                connection
                    .query_row(
                        "SELECT id, active_device_id, owner_session_id
                         FROM sister_worktree_groups
                         WHERE repository_id=?1 AND branch_ref=?2",
                        params![repository_id, branch_ref],
                        |row| {
                            Ok(SisterWorktreeLock {
                                sister_group_id: row.get(0)?,
                                active_device_id: row.get(1)?,
                                owner_session_id: row.get(2)?,
                            })
                        },
                    )
                    .optional()
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.into_string())
    }

    /// Moves editing authority to a destination Session instance created by a device move. A
    /// group owned by any other Session is left untouched.
    pub(crate) fn transfer_owner(
        &self,
        repository_id: &str,
        branch_ref: &str,
        from_session_id: &str,
        to_session_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), String> {
        self.0
            .write("transfer sister worktree owner", |transaction| {
                transaction
                    .execute(
                        "UPDATE sister_worktree_groups SET owner_session_id=?4, updated_at=?5
                         WHERE repository_id=?1 AND branch_ref=?2 AND owner_session_id=?3",
                        params![
                            repository_id,
                            branch_ref,
                            from_session_id,
                            to_session_id,
                            at.to_rfc3339()
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn activate(
        &self,
        source: &SessionExecutionTarget,
        destination: &SessionExecutionTarget,
        session_id: &str,
        at: DateTime<Utc>,
    ) -> Result<String, String> {
        if source.repository_id != destination.repository_id || source.branch_ref != destination.branch_ref {
            return Err("Sister worktrees must share a repository and branch".into());
        }
        let repository_id = source.repository_id.clone();
        let branch_ref = source.branch_ref.clone();
        self.0
            .write("activate sister worktree", |transaction| {
                let current: Option<(String, String)> = transaction
                    .query_row(
                        "SELECT id, owner_session_id FROM sister_worktree_groups
                         WHERE repository_id=?1 AND branch_ref=?2",
                        params![repository_id, branch_ref],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                let group_id = match current {
                    Some((_id, owner)) if owner != session_id => {
                        return Err(format!(
                            "This sister worktree group is locked by Session {owner}"
                        ));
                    }
                    Some((id, _)) => id,
                    None => uuid::Uuid::new_v4().to_string(),
                };
                transaction
                    .execute(
                        "INSERT INTO sister_worktree_groups
                         (id,repository_id,branch_ref,active_device_id,owner_session_id,updated_at)
                         VALUES(?1,?2,?3,?4,?5,?6)
                         ON CONFLICT(repository_id,branch_ref) DO UPDATE SET
                           active_device_id=excluded.active_device_id,
                           owner_session_id=excluded.owner_session_id,
                           updated_at=excluded.updated_at",
                        params![
                            group_id,
                            repository_id,
                            branch_ref,
                            destination.execution.device_id,
                            session_id,
                            at.to_rfc3339(),
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                for target in [source, destination] {
                    transaction
                        .execute(
                            "INSERT INTO sister_worktree_instances
                             (sister_group_id,device_id,worktree_id,path,head,branch_ref)
                             VALUES(?1,?2,?3,?4,?5,?6)
                             ON CONFLICT(sister_group_id,device_id,worktree_id) DO UPDATE SET
                               path=excluded.path, head=excluded.head, branch_ref=excluded.branch_ref",
                            params![
                                group_id,
                                target.execution.device_id,
                                target.worktree_id,
                                target.path,
                                target.head,
                                target.branch_ref,
                            ],
                        )
                        .map_err(|error| error.to_string())?;
                }
                Ok(group_id)
            })
            .map_err(|error| error.into_string())
    }

    /// Claims the source as the active member while its Session has a pending device move.
    /// That makes the selection lock visible before the destination has been materialized.
    pub(crate) fn claim(
        &self,
        source: &SessionExecutionTarget,
        session_id: &str,
        at: DateTime<Utc>,
    ) -> Result<String, String> {
        self.0
            .write("claim sister worktree", |transaction| {
                let current: Option<(String, String)> = transaction
                    .query_row(
                        "SELECT id, owner_session_id FROM sister_worktree_groups
                         WHERE repository_id=?1 AND branch_ref=?2",
                        params![source.repository_id, source.branch_ref],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                let group_id = match current {
                    Some((_id, owner)) if owner != session_id => {
                        return Err(format!(
                            "This sister worktree group is locked by Session {owner}"
                        ));
                    }
                    Some((id, _)) => id,
                    None => uuid::Uuid::new_v4().to_string(),
                };
                transaction
                    .execute(
                        "INSERT INTO sister_worktree_groups
                         (id,repository_id,branch_ref,active_device_id,owner_session_id,updated_at)
                         VALUES(?1,?2,?3,?4,?5,?6)
                         ON CONFLICT(repository_id,branch_ref) DO UPDATE SET
                           active_device_id=excluded.active_device_id,
                           owner_session_id=excluded.owner_session_id,
                           updated_at=excluded.updated_at",
                        params![
                            group_id,
                            source.repository_id,
                            source.branch_ref,
                            source.execution.device_id,
                            session_id,
                            at.to_rfc3339(),
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                transaction
                    .execute(
                        "INSERT INTO sister_worktree_instances
                         (sister_group_id,device_id,worktree_id,path,head,branch_ref)
                         VALUES(?1,?2,?3,?4,?5,?6)
                         ON CONFLICT(sister_group_id,device_id,worktree_id) DO UPDATE SET
                           path=excluded.path, head=excluded.head, branch_ref=excluded.branch_ref",
                        params![
                            group_id,
                            source.execution.device_id,
                            source.worktree_id,
                            source.path,
                            source.head,
                            source.branch_ref,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(group_id)
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn contains_instance(
        &self,
        sister_group_id: &str,
        device_id: &str,
        worktree_id: &str,
    ) -> Result<bool, String> {
        self.0
            .read("read sister worktree instance", |connection| {
                connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sister_worktree_instances
                         WHERE sister_group_id=?1 AND device_id=?2 AND worktree_id=?3)",
                        params![sister_group_id, device_id, worktree_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.into_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        execution_targets::domain::{ExecutionBinding, ExecutionConnection},
        persistence::ActiveDatabase,
    };
    use rusqlite::Connection;

    fn target(device: &str) -> SessionExecutionTarget {
        SessionExecutionTarget {
            capability_profile_id: format!("profile-{device}"),
            capability_profile_revision: 1,
            execution: ExecutionBinding {
                device_id: device.into(),
                device_name: device.into(),
                provider: "codex".into(),
                configuration_ref: "selected".into(),
                connection: if device == "laptop" {
                    ExecutionConnection::Local
                } else {
                    ExecutionConnection::Ssh { target: "host".into(), host_executable: "orchid-host".into() }
                },
            },
            repository_id: "repository".into(),
            branch_ref: "refs/heads/feature".into(),
            worktree_id: format!("{device}-worktree"),
            path: format!("/{device}/worktree"),
            head: Some("abc".into()),
        }
    }

    #[test]
    fn moving_a_sister_keeps_both_instances_and_rejects_another_session() {
        let database = Arc::new(
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection.execute_batch(SCHEMA).map_err(|error| error.to_string())
            })
            .unwrap(),
        );
        let store = SisterWorktreeStore::new(database);
        let source = target("laptop");
        let destination = target("server");
        let group = store.claim(&source, "session-a", Utc::now()).unwrap();
        assert_eq!(
            store.lock_for("repository", "refs/heads/feature").unwrap(),
            Some(SisterWorktreeLock {
                sister_group_id: group.clone(),
                active_device_id: "laptop".into(),
                owner_session_id: "session-a".into(),
            })
        );
        assert!(store
            .contains_instance(&group, "laptop", "laptop-worktree")
            .unwrap());
        assert_eq!(
            store
                .activate(&source, &destination, "session-a", Utc::now())
                .unwrap(),
            group
        );
        assert_eq!(
            store.lock_for("repository", "refs/heads/feature").unwrap(),
            Some(SisterWorktreeLock {
                sister_group_id: group,
                active_device_id: "server".into(),
                owner_session_id: "session-a".into(),
            })
        );
        assert!(store
            .activate(&destination, &source, "session-b", Utc::now())
            .unwrap_err()
            .contains("session-a"));
    }
}
