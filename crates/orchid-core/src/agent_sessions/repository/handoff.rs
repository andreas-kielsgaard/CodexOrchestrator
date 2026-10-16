//! Explicit online ownership transfer; replicas never acquire authority by being read.
use super::*;
use crate::synchronization::{OwnershipTransfer, SessionChange};

pub(super) fn transfers_from(
    db: &Connection,
    id: &AgentSessionId,
) -> Result<Vec<OwnershipTransfer>, RepositoryError> {
    let json: Option<String> = db
        .query_row(
            "SELECT transfers_json FROM orchid_session_authority WHERE session_id=?1",
            [id.as_str()],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql_unavailable("read session ownership"))?;
    json.map(|s| from_json(&s, "ownership transfers"))
        .transpose()
        .map(|v| v.unwrap_or_default())
}
pub(super) fn owns(db: &Connection, id: &AgentSessionId) -> Result<bool, RepositoryError> {
    db.query_row("SELECT NOT EXISTS(SELECT 1 FROM orchid_session_authority WHERE session_id=?1 AND owner!=(SELECT device_id FROM orchid_instance WHERE singleton=1))",[id.as_str()],|r|r.get(0)).map_err(sql_unavailable("read execution ownership"))
}
pub(super) fn release(
    db: &Connection,
    operation: &crate::agent_sessions::target_operation::SessionTargetOperation,
) -> Result<(), RepositoryError> {
    let target = operation
        .staged_target
        .as_ref()
        .ok_or_else(|| not_found("Staged target unavailable"))?;
    if !matches!(
        target.execution.connection,
        crate::execution_targets::domain::ExecutionConnection::Device
    ) {
        return Ok(());
    }
    if !owns(db, &operation.session_id)? {
        return Err(RepositoryError::new(
            RepositoryErrorKind::Conflict,
            "Session execution belongs to another device",
        ));
    }
    let source: String = db
        .query_row(
            "SELECT device_id FROM orchid_instance WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(sql_unavailable("read source device"))?;
    let mut transfers = transfers_from(db, &operation.session_id)?;
    transfers.push(OwnershipTransfer {
        operation_id: operation.id.clone(),
        source,
        destination: target.execution.device_id.clone(),
    });
    db.execute("INSERT INTO orchid_session_authority VALUES(?1,?2,?3,1) ON CONFLICT(session_id) DO UPDATE SET owner=excluded.owner,transfers_json=excluded.transfers_json,delivery_pending=1",params![operation.session_id.as_str(),target.execution.device_id,to_json(&transfers)?]).map_err(sql_write("release session ownership"))?;
    Ok(())
}

impl SqliteAgentSessionRepository {
    pub fn handoff_accepted(&self, change: &SessionChange) -> Result<bool, String> {
        self.read("check accepted handoff", |db| {
            Ok(
                transfers_from(db, &change.session.history.session.id)? == change.transfers
                    && owns(db, &change.session.history.session.id)?
                    && get_session_from(db, &change.session.history.session.id)?.is_some(),
            )
        })
        .map_err(|e| e.to_string())
    }
    pub fn execution_owner(&self, id: &AgentSessionId) -> Result<Option<String>, String> {
        self.database
            .read("read session owner", |db| {
                db.query_row(
                    "SELECT owner FROM orchid_session_authority WHERE session_id=?1",
                    [id.as_str()],
                    |r| r.get(0),
                )
                .optional()
            })
            .map_err(|e| e.to_string())
    }
    pub fn pending_handoffs(&self) -> Result<Vec<(String, String)>, String> {
        self.database.read("read outgoing handoffs",|db|db.prepare("SELECT session_id,owner FROM orchid_session_authority WHERE delivery_pending=1")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<Result<Vec<_>,rusqlite::Error>>()).map_err(|e|e.to_string())
    }
    pub fn acknowledge_handoff(&self, id: &str, destination: &str) -> Result<(), String> {
        self.database.write("acknowledge ownership transfer",|db|db.execute("UPDATE orchid_session_authority SET delivery_pending=0 WHERE session_id=?1 AND owner=?2",params![id,destination])).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn accept_handoff(&self, device: &str, change: &SessionChange) -> Result<(), String> {
        let transfer = change
            .transfers
            .last()
            .ok_or("Missing ownership transfer")?;
        if transfer.destination != device || transfer.source != change.device_id {
            return Err("Ownership transfer addresses another device".into());
        }
        if self.handoff_accepted(change)? {
            return Ok(());
        }
        for pair in change.transfers.windows(2) {
            if pair[0].destination != pair[1].source {
                return Err("Invalid ownership history".into());
            }
        }
        if change
            .session
            .history
            .invocations
            .iter()
            .any(|t| t.invocation.status == AgentInvocationStatus::Running)
        {
            return Err("Cannot transfer a running invocation".into());
        }
        let mut session = change.session.history.session.clone();
        let id = session.id.clone();
        let target = session
            .execution_target
            .as_mut()
            .ok_or("Transferred conversation has no worktree")?;
        if target.execution.device_id != device {
            return Err("Transferred target belongs to another device".into());
        }
        if !std::path::Path::new(&target.path).is_dir() {
            return Err("Transferred worktree is unavailable".into());
        }
        target.execution.device_id = "local".into();
        target.execution.connection = crate::execution_targets::domain::ExecutionConnection::Local;
        session.runtime_binding = AgentRuntimeBinding {
            external_context_id: None,
            runtime_version: None,
        };
        session.session_profile = None;
        session.harness_version = None;
        self.write("accept execution ownership",|tx|{
            let previous=transfers_from(tx,&id)?;
            if previous==change.transfers && get_session_from(tx,&id)?.is_some() && owns(tx,&id)?{return Ok(());}
            if !change.transfers.starts_with(&previous) {return Err(RepositoryError::new(RepositoryErrorKind::Conflict,"Conflicting ownership transfer"));}
            if get_session_from(tx,&id)?.is_some() && owns(tx,&id)? {return Err(RepositoryError::new(RepositoryErrorKind::Conflict,"Session already executes on this device"));}
            // Ownership and queue import commit together. Native homes and provider handles do not move.
            tx.execute("INSERT INTO orchid_session_authority VALUES(?1,?2,?3,0) ON CONFLICT(session_id) DO UPDATE SET owner=excluded.owner,transfers_json=excluded.transfers_json,delivery_pending=0",params![id.as_str(),device,to_json(&change.transfers)?]).map_err(sql_write("accept session authority"))?;
            if get_session_from(tx,&id)?.is_none(){insert_session(tx,&session)?;}else{
                tx.execute("UPDATE agent_sessions SET title=?2,availability=?3,external_context_id=NULL,runtime_version=NULL,working_directory=?4,execution_target_json=?5,requested_options_json=?6,session_profile_json=NULL,harness_version_ref_json=NULL,assigned_identity_json=?7,updated_at=?8,binding_owner_json=NULL WHERE id=?1",params![id.as_str(),session.title,availability_text(session.availability),session.working_directory,to_json(&session.execution_target)?,to_json(&session.requested_options)?,session.assigned_identity.as_ref().map(to_json).transpose()?,timestamp(session.updated_at)]).map_err(sql_write("update transferred session"))?;
            }
            tx.execute("DELETE FROM agent_session_queue_entries WHERE session_id=?1",[id.as_str()]).map_err(sql_write("replace transferred queue"))?;
            tx.execute("DELETE FROM agent_session_preparations WHERE session_id=?1",[id.as_str()]).map_err(sql_write("replace transferred preparations"))?;
            tx.execute("DELETE FROM agent_session_current_execution WHERE session_id=?1",[id.as_str()]).map_err(sql_write("clear previous execution resolution"))?;
            for turn in &change.session.history.invocations {
                if turn.invocation.session_id!=id {return Err(RepositoryError::new(RepositoryErrorKind::InvalidState,"Foreign invocation in handoff"));}
                if get_invocation_from(tx,&turn.invocation.id)?.is_none(){insert_invocation(tx,&turn.invocation)?;}else{update_invocation(tx,&turn.invocation)?;}
                tx.execute("UPDATE agent_session_invocations SET submitted_text=?2,requested_options_json=?3,attachments_json=?4 WHERE id=?1",params![turn.invocation.id.as_str(),turn.invocation.submitted_text,to_json(&turn.invocation.requested_options)?,to_json(&turn.invocation.attachments)?]).map_err(sql_write("update transferred message"))?;
                if turn.invocation.status!=AgentInvocationStatus::Pending {
                    let origin=change.session.invocation_devices.get(turn.invocation.id.as_str()).unwrap_or(&change.device_id);
                    tx.execute("INSERT INTO orchid_invocation_devices VALUES(?1,?2) ON CONFLICT(invocation_id) DO UPDATE SET device_id=excluded.device_id",params![turn.invocation.id.as_str(),origin]).map_err(sql_write("retain invocation execution device"))?;
                }
                for event in &turn.events {
                    if event.invocation_id!=turn.invocation.id{return Err(RepositoryError::new(RepositoryErrorKind::InvalidState,"Foreign event in handoff"));}
                    let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM agent_session_runtime_events WHERE id=?1)",[event.id.as_str()],|r|r.get(0)).map_err(sql_unavailable("read transferred event"))?;
                    if !exists{insert_event(tx,event)?;super::file_history::append(tx,event)?;}
                }
                if let Some(at)=turn.launch_accepted_at{tx.execute("INSERT OR IGNORE INTO agent_session_invocation_launch_acceptances VALUES(?1,?2)",params![turn.invocation.id.as_str(),timestamp(at)]).map_err(sql_write("copy launch evidence"))?;}
                if let Some(p)=&turn.import_provenance{tx.execute("INSERT OR IGNORE INTO agent_session_imported_turns VALUES(?1,?2,?3,?4,?5,?6)",params![turn.invocation.id.as_str(),id.as_str(),p.source_turn_id,p.ordinal,p.source_started_at,p.source_completed_at]).map_err(sql_write("copy imported turn provenance"))?;}
            }
            for op in &change.session.target_operations{
                let mut value=serde_json::to_value(op).map_err(|e|RepositoryError::new(RepositoryErrorKind::InvalidState,e.to_string()))?;
                if matches!(op.phase,crate::agent_sessions::target_operation::TargetOperationPhase::Pending|crate::agent_sessions::target_operation::TargetOperationPhase::Blocked){crate::synchronization::definitions::portable_routes(&mut value,device,true);}
                tx.execute("INSERT INTO agent_session_target_operations VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET payload_json=excluded.payload_json",params![op.id,id.as_str(),value.to_string()]).map_err(sql_write("copy target operations"))?;
            }
            for preparation in &change.session.preparations{
                let mut p=preparation.clone();
                if let Some(snapshot)=&p.execution_snapshot {
                    tx.execute("INSERT INTO agent_session_invocation_execution_snapshots VALUES(?1,?2,?3) ON CONFLICT(invocation_id) DO UPDATE SET snapshot_json=excluded.snapshot_json",params![p.invocation_id.as_str(),to_json(snapshot)?,timestamp(Utc::now())]).map_err(sql_write("retain execution observation"))?;
                }
                if !p.delivery_started {
                    p.pinned_delivery=false;
                    p.selection=None;p.source_target=None;p.source_provider_binding=None;p.resolved_target=None;p.prepared_binding=None;p.resolution=None;p.current_resolution=None;p.execution_snapshot=None;
                    p.source_binding=session.runtime_binding.clone();p.resolved_working_directory=None;p.requested_capability_profile=None;
                    p.phase=crate::agent_sessions::preparation::PreparationPhase::Accepted;
                }
                tx.execute("INSERT INTO agent_session_preparations VALUES(?1,?2,?3)",params![p.invocation_id.as_str(),id.as_str(),to_json(&p)?]).map_err(sql_write("copy pending preparations"))?;
            }
            for entry in &change.session.queue.entries{super::queue::insert_entry(tx,entry)?;}
            if let Some(address)=&change.session.address{address.import(tx,&id)?;}
            if let Some(fork)=&change.session.fork{tx.execute("INSERT INTO agent_session_forks VALUES(?1,?2,?3,?4) ON CONFLICT(session_id) DO NOTHING",params![id.as_str(),fork.source_session_id,fork.source_invocation_id,fork.created_at]).map_err(sql_write("retain fork lineage"))?;}
            tx.execute("INSERT INTO agent_session_queue_state VALUES(?1,?2,?3,?3) ON CONFLICT(session_id) DO UPDATE SET revision=excluded.revision,updated_at=excluded.updated_at",params![id.as_str(),change.session.queue.revision,timestamp(Utc::now())]).map_err(sql_write("copy queue revision"))?;
            tx.execute("DELETE FROM agent_session_history_positions WHERE session_id=?1",[id.as_str()]).map_err(sql_write("replace timeline positions"))?;
            for p in &change.session.history.timeline.positions{tx.execute("INSERT INTO agent_session_history_positions VALUES(?1,?2,?3,?4)",params![id.as_str(),p.ordinal,p.invocation_id,p.operation_id]).map_err(sql_write("copy timeline positions"))?;}
            if let Some(prefix)=&change.session.initial_prompt_prefix{tx.execute("INSERT INTO agent_session_initial_prompt_prefixes VALUES(?1,?2,?3) ON CONFLICT(session_id) DO UPDATE SET prefix_json=excluded.prefix_json",params![id.as_str(),to_json(prefix)?,timestamp(Utc::now())]).map_err(sql_write("copy initial context"))?;}
            tx.execute("DELETE FROM orchid_session_replicas WHERE session_id=?1",[id.as_str()]).map_err(sql_write("retire previous replica"))?;
            tx.execute("DELETE FROM orchid_session_replica_summaries WHERE session_id=?1",[id.as_str()]).map_err(sql_write("retire previous replica summary"))?;
            Ok(())
        }).map_err(|e|e.to_string())
    }
}
