//! Organization writes never update Session activity, execution metadata, or addresses.
use super::{mapping::*, SqliteAgentSessionRepository};
use crate::agent_sessions::{
    domain::{validate_session, AgentSession, AgentSessionId},
    organization::{SessionOrganization, SessionPlacement},
    ports::RepositoryError,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_organization (
    session_id TEXT PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
    placement_kind TEXT NOT NULL DEFAULT 'default',
    placement_target_id TEXT,
    pinned_at TEXT,
    CHECK ((placement_kind IN ('default','unfiled') AND placement_target_id IS NULL)
        OR (placement_kind IN ('repository','workflow_instance') AND length(placement_target_id) > 0 AND placement_target_id IS NOT NULL))
);
"#;

fn placement_columns(placement: &SessionPlacement) -> (&str, Option<&str>) {
    match placement {
        SessionPlacement::Default => ("default", None),
        SessionPlacement::Unfiled => ("unfiled", None),
        SessionPlacement::Repository { repository_id } => ("repository", Some(repository_id)),
        SessionPlacement::WorkflowInstance { instance_id } => {
            ("workflow_instance", Some(instance_id))
        }
    }
}
pub(crate) fn write_placement(
    connection: &Connection,
    session_id: &AgentSessionId,
    placement: &SessionPlacement,
) -> Result<(), RepositoryError> {
    let (kind, target) = placement_columns(placement);
    connection.execute("INSERT INTO agent_session_organization(session_id,placement_kind,placement_target_id) VALUES(?1,?2,?3) ON CONFLICT(session_id) DO UPDATE SET placement_kind=excluded.placement_kind,placement_target_id=excluded.placement_target_id", params![session_id.as_str(),kind,target])
        .map_err(sql_write("store Session placement"))?;
    Ok(())
}
impl SqliteAgentSessionRepository {
    pub(crate) fn create_session_with_placement(
        &self,
        session: AgentSession,
        placement: &SessionPlacement,
    ) -> Result<AgentSession, RepositoryError> {
        validate_session(&session).map_err(contract_error)?;
        self.write("create organized Agent Session", |transaction| {
            insert_session(transaction, &session)?;
            write_placement(transaction, &session.id, placement)?;
            Ok(session)
        })
    }
    pub(crate) fn move_session(
        &self,
        id: &AgentSessionId,
        placement: &SessionPlacement,
    ) -> Result<(), RepositoryError> {
        self.write("move Agent Session", |transaction| {
            write_placement(transaction, id, placement)
        })
    }
    pub(crate) fn pin_session(
        &self,
        id: &AgentSessionId,
        pinned: bool,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.write("pin Agent Session", |transaction| {
            transaction.execute("INSERT INTO agent_session_organization(session_id,pinned_at) VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET pinned_at=CASE WHEN excluded.pinned_at IS NULL THEN NULL ELSE COALESCE(agent_session_organization.pinned_at,excluded.pinned_at) END", params![id.as_str(),pinned.then(|| timestamp(now))])
                .map_err(sql_write("store Session pin"))?;
            Ok(())
        })
    }
    pub(crate) fn list_organization(&self) -> Result<Vec<SessionOrganization>, RepositoryError> {
        self.read("list Session organization", |connection| {
            let mut query = connection.prepare("SELECT session_id,placement_kind,placement_target_id,pinned_at FROM agent_session_organization").map_err(sql_unavailable("read Session organization"))?;
            let rows = query.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<String>>(2)?,row.get::<_,Option<String>>(3)?)))
                .map_err(sql_unavailable("query Session organization"))?;
            rows.map(|row| {
                let (id, kind, target, pin) = row.map_err(sql_unavailable("read Session organization row"))?;
                let placement = match kind.as_str() {
                    "default" => SessionPlacement::Default,
                    "unfiled" => SessionPlacement::Unfiled,
                    "repository" => SessionPlacement::Repository { repository_id: target.unwrap_or_default() },
                    "workflow_instance" => SessionPlacement::WorkflowInstance { instance_id: target.unwrap_or_default() },
                    _ => return Err(not_found("Unknown Session placement")),
                };
                Ok(SessionOrganization { session_id: AgentSessionId::new(id).map_err(contract_error)?, placement,
                    pinned_at: pin.map(|value| DateTime::parse_from_rfc3339(&value).map(|time| time.with_timezone(&Utc)).map_err(|_| not_found("Invalid pin timestamp"))).transpose()? })
            }).collect()
        })
    }
}
