use super::*;
use crate::agent_sessions::file_history::{FileOperation, ReportedFileChange, SessionFileChange};
use crate::session_events::{ReferenceIdentity, SessionLogicalAddress};

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_file_changes (
    runtime_event_id TEXT NOT NULL REFERENCES agent_session_runtime_events(id) ON DELETE CASCADE,
    change_index INTEGER NOT NULL,
    path TEXT NOT NULL,
    operation TEXT NOT NULL CHECK(operation IN ('create','edit','delete')),
    PRIMARY KEY(runtime_event_id, change_index)
);
"#;

pub(super) fn append(
    connection: &Connection,
    event: &AgentRuntimeEvent,
) -> Result<(), RepositoryError> {
    let Some(value) = event
        .normalized
        .as_ref()
        .and_then(|n| n.details.as_ref())
        .and_then(|d| d.get("fileChanges"))
    else {
        return Ok(());
    };
    let changes: Vec<ReportedFileChange> =
        serde_json::from_value(value.clone()).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorKind::InvalidState,
                format!("Invalid normalized file changes: {error}"),
            )
        })?;
    for (index, change) in changes.iter().enumerate() {
        if change.path.trim().is_empty() {
            return Err(RepositoryError::new(
                RepositoryErrorKind::InvalidState,
                "A reported file path is empty",
            ));
        }
        let operation = match change.operation {
            FileOperation::Create => "create",
            FileOperation::Edit => "edit",
            FileOperation::Delete => "delete",
        };
        connection.execute("INSERT INTO agent_session_file_changes(runtime_event_id,change_index,path,operation) VALUES(?1,?2,?3,?4)",
            params![event.id.as_str(), index as i64, change.path, operation])
            .map_err(sql_write("record session file change"))?;
    }
    Ok(())
}

pub(super) fn query(
    connection: &Connection,
    scope: &ReferenceIdentity,
) -> Result<Vec<SessionFileChange>, RepositoryError> {
    let mut statement = connection.prepare(
        "SELECT a.subject_namespace,a.subject_kind,a.subject_id,s.id,i.id,s.working_directory,c.path,c.operation,e.recorded_at
         FROM agent_session_file_changes c
         JOIN agent_session_runtime_events e ON e.id=c.runtime_event_id
         JOIN agent_session_invocations i ON i.id=e.invocation_id
         JOIN agent_sessions s ON s.id=i.session_id
         JOIN agent_session_addresses a ON a.session_id=s.id
         WHERE a.scope_namespace=?1 AND a.scope_kind=?2 AND a.scope_id=?3
         ORDER BY e.recorded_at,e.rowid,c.change_index")
        .map_err(sql_unavailable("prepare session file history"))?;
    let rows = statement
        .query_map(
            params![scope.namespace(), scope.kind(), scope.id()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .map_err(sql_unavailable("query session file history"))?;
    rows.map(|row| {
        let (
            namespace,
            kind,
            id,
            session_id,
            invocation_id,
            working_directory,
            path,
            operation,
            recorded_at,
        ) = row.map_err(sql_unavailable("read session file change"))?;
        let subject = ReferenceIdentity::new(namespace, kind, id).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorKind::InvalidState,
                format!("Invalid file change address: {error}"),
            )
        })?;
        let operation = match operation.as_str() {
            "create" => FileOperation::Create,
            "edit" => FileOperation::Edit,
            "delete" => FileOperation::Delete,
            _ => {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::InvalidState,
                    "Invalid stored file operation",
                ))
            }
        };
        Ok(SessionFileChange {
            address: SessionLogicalAddress::new(scope.clone(), subject),
            session_id,
            invocation_id,
            working_directory,
            path,
            operation,
            recorded_at,
        })
    })
    .collect()
}
