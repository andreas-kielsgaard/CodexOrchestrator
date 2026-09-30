use super::queue::{increment_revision, read_snapshot, require_revision};
use super::*;
use crate::agent_sessions::{
    queue::SessionQueueSnapshot,
    target_operation::{validate_target_operation, SessionTargetOperation},
};

impl SqliteAgentSessionRepository {
    pub(super) fn read_target_operation_record(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionTargetOperation>, RepositoryError> {
        self.read("read target operation", |connection| {
            let json: Option<String> = connection
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [operation_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_unavailable("read target operation"))?;
            json.map(|value| from_json(&value, "target operation"))
                .transpose()
        })
    }
    pub(super) fn save_target_operation_record(
        &self,
        operation: &SessionTargetOperation,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        validate_target_operation(operation)
            .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
        self.write("save target operation", |tx| {
            let changed = tx
                .execute(
                    "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1 AND session_id=?3",
                    params![operation.id,to_json(operation)?,operation.session_id.as_str()],
                )
                .map_err(sql_write("save target operation"))?;
            if changed != 1 {
                return Err(not_found("Target operation not found"));
            }
            increment_revision(tx, &operation.session_id, operation.updated_at)?;
            read_snapshot(tx, &operation.session_id)
        })
    }
    pub(super) fn replace_target_operation_record(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
        expected_revision: u64,
        request: crate::agent_sessions::target_operation::TargetSwitchRequest,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("edit queued target operation", |tx| {
            require_revision(tx, session_id, expected_revision)?;
            let (operation_id, state, ordinal): (String, String, i64) = tx
                .query_row(
                    "SELECT operation_id,state,ordinal FROM agent_session_queue_entries WHERE id=?1 AND session_id=?2 AND kind='target_operation'",
                    params![entry_id,session_id.as_str()],
                    |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
                )
                .map_err(sql_unavailable("load queued target operation"))?;
            if !matches!(state.as_str(), "pending" | "blocked") {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Target activation has already started",
                ));
            }
            let json: String = tx
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [&operation_id],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("read target operation"))?;
            let mut operation: SessionTargetOperation = from_json(&json, "target operation")?;
            operation.request = request;
            operation.needs_resolution = false;
            operation.phase = crate::agent_sessions::target_operation::TargetOperationPhase::Pending;
            operation.actions.clear();
            operation.projected_plan.stale = true;
            operation.staged_target = None;
            operation.error = None;
            operation.generation = operation.generation.saturating_add(1);
            operation.updated_at = at;
            validate_target_operation(&operation)
                .map_err(|error| RepositoryError::new(RepositoryErrorKind::InvalidState, error))?;
            tx.execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![operation_id,to_json(&operation)?],
            )
            .map_err(sql_write("edit target operation"))?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='pending',blocked_reason=NULL,updated_at=?2 WHERE id=?1",
                params![entry_id,timestamp(at)],
            )
            .map_err(sql_write("reset target operation queue entry"))?;
            super::queue::mark_projections_stale(tx, session_id, ordinal)?;
            increment_revision(tx, session_id, at)?;
            read_snapshot(tx, session_id)
        })
    }
    pub(super) fn activate_target_operation_record(
        &self,
        operation_id: &str,
        at: DateTime<Utc>,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("activate queued target", |tx| {
            let json: String = tx
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [operation_id],
                    |row| row.get(0),
                )
                .map_err(sql_unavailable("read target operation"))?;
            let mut operation: SessionTargetOperation = from_json(&json, "target operation")?;
            if !matches!(operation.phase, crate::agent_sessions::target_operation::TargetOperationPhase::Staged | crate::agent_sessions::target_operation::TargetOperationPhase::Activating) {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Target operation is not staged",
                ));
            }
            let target = operation
                .staged_target
                .clone()
                .ok_or_else(|| not_found("Staged target unavailable"))?;
            required_session(tx, &operation.session_id)?;
            tx.execute(
                "UPDATE agent_sessions SET execution_target_json=?2,working_directory=?3,workspace_origin='explicit',updated_at=?4 WHERE id=?1",
                params![operation.session_id.as_str(),to_json(&target)?,target.path,timestamp(at)],
            )
            .map_err(sql_write("activate Session target"))?;
            operation.phase = crate::agent_sessions::target_operation::TargetOperationPhase::Completed;
            operation.error = None;
            operation.updated_at = at;
            for action in &mut operation.actions {
                if matches!(
                    action.kind,
                    crate::agent_sessions::target_operation::TargetOperationActionKind::ActivateTarget
                ) {
                    action.status = crate::agent_sessions::target_operation::TargetOperationActionStatus::Completed;
                    action.error = None;
                }
            }
            operation.receipt = Some(crate::agent_sessions::history::WorkspaceTransitionReceipt { operation_id: operation.id.clone(), intent: operation.request.intent, source: operation.source_target.clone(), destination: target.clone(), actions: operation.actions.clone(), completed_at: at });
            super::history::allocate_position(tx,&operation.session_id,None,Some(operation_id))?;
            tx.execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![operation_id,to_json(&operation)?],
            )
            .map_err(sql_write("complete target operation"))?;
            tx.execute(
                "UPDATE agent_session_queue_entries SET state='completed',blocked_reason=NULL,updated_at=?2 WHERE operation_id=?1",
                params![operation_id,timestamp(at)],
            )
            .map_err(sql_write("complete target operation queue entry"))?;
            increment_revision(tx, &operation.session_id, at)?;
            read_snapshot(tx, &operation.session_id)
        })
    }
    pub(super) fn save_target_projection_record(
        &self,
        operation: &SessionTargetOperation,
        revision: u64,
    ) -> Result<SessionQueueSnapshot, RepositoryError> {
        self.write("recalculate target projection", |tx| {
            require_revision(tx, &operation.session_id, revision)?;
            let current: String = tx
                .query_row(
                    "SELECT payload_json FROM agent_session_target_operations WHERE id=?1",
                    [&operation.id],
                    |r| r.get(0),
                )
                .map_err(sql_unavailable("read target operation"))?;
            let mut current: SessionTargetOperation = from_json(&current, "target operation")?;
            if current.generation != operation.generation {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "Target request changed",
                ));
            }
            current.projected_plan = operation.projected_plan.clone();
            tx.execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![current.id, to_json(&current)?],
            )
            .map_err(sql_write("save target projection"))?;
            increment_revision(tx, &operation.session_id, operation.updated_at)?;
            read_snapshot(tx, &operation.session_id)
        })
    }
    pub(super) fn discard_target_operation_record(
        &self,
        id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.write("finish no-action switch", |tx| {
            let session: String = tx
                .query_row(
                    "SELECT session_id FROM agent_session_target_operations WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(sql_unavailable("read switch session"))?;
            tx.execute(
                "DELETE FROM agent_session_target_operations WHERE id=?1",
                [id],
            )
            .map_err(sql_write("remove no-action switch"))?;
            increment_revision(
                tx,
                &AgentSessionId::new(session).map_err(|e| contract_error(e))?,
                at,
            )?;
            Ok(())
        })
    }
}

pub(crate) fn upgrade_target_switches(connection: &Connection) -> Result<(), String> {
    super::schema::ensure_agent_session_ownership_schema(connection)?;
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(agent_sessions)")
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if !columns.iter().any(|c| c == "binding_owner_json") {
        connection.execute_batch("ALTER TABLE agent_sessions ADD COLUMN binding_owner_json TEXT CHECK(binding_owner_json IS NULL OR json_valid(binding_owner_json));").map_err(|e|e.to_string())?;
    }
    connection.execute_batch("UPDATE agent_sessions SET binding_owner_json=json_object('execution',json_extract(execution_target_json,'$.execution'),'workspace',json_object('repositoryId',json_extract(execution_target_json,'$.repositoryId'),'worktreeId',json_extract(execution_target_json,'$.worktreeId'),'path',json_extract(execution_target_json,'$.path'))) WHERE binding_owner_json IS NULL AND external_context_id IS NOT NULL AND execution_target_json IS NOT NULL;").map_err(|e|e.to_string())?;
    let records: Vec<(String,String)> = connection.prepare("SELECT id,payload_json FROM agent_session_target_operations WHERE json_type(payload_json,'$.request') IS NULL").map_err(|e|e.to_string())?.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())?;
    for (id, json) in records {
        let mut value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let destination = value["destinationSelection"].take();
        value["request"] = serde_json::json!({"destination":destination,"intent":"change_conversation_target","migrationResolution":null});
        let unfinished = !matches!(value["phase"].as_str(), Some("completed" | "canceled"));
        value["needsResolution"] = unfinished.into();
        value["receipt"] = serde_json::Value::Null;
        value["projectedPlan"] = serde_json::json!({"calculatedAt":value["updatedAt"],"requestRevision":value["generation"],"stale":true,"source":value["sourceTarget"],"facts":{"destination":null,"destinationReady":null,"unknown":["Choose intent to calculate the projected work"],"blockers":[]},"steps":[]});
        if let Some(actions) = value["actions"].as_array_mut() {
            actions.retain(|a| a["kind"] != "select_provider_context");
        }
        if unfinished {
            value["phase"] = "blocked".into();
            value["error"] =
                "Choose the intent and migration resolution for this older switch".into();
        }
        value
            .as_object_mut()
            .unwrap()
            .remove("destinationSelection");
        value.as_object_mut().unwrap().remove("activatedBinding");
        connection
            .execute(
                "UPDATE agent_session_target_operations SET payload_json=?2 WHERE id=?1",
                params![id, value.to_string()],
            )
            .map_err(|e| e.to_string())?;
        if unfinished {
            connection.execute("UPDATE agent_session_queue_entries SET state='blocked',blocked_reason='Choose the intent and migration resolution for this older switch' WHERE operation_id=?1",[&id]).map_err(|e|e.to_string())?;
        }
    }
    connection.execute_batch("DELETE FROM agent_session_invocations WHERE id IN (SELECT invocation_id FROM agent_session_queue_entries WHERE state='canceled') AND id NOT IN (SELECT invocation_id FROM agent_session_invocation_launch_acceptances); DELETE FROM agent_session_target_operations WHERE id IN (SELECT operation_id FROM agent_session_queue_entries WHERE state='canceled');").map_err(|e|e.to_string())?;
    Ok(())
}
