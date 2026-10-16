use super::*;
use crate::synchronization::{SessionChange, SessionReplica};
impl SqliteAgentSessionRepository {
    pub(crate) fn database(&self) -> Arc<ActiveDatabase> {
        self.database.clone()
    }
    pub fn definitions(&self) -> crate::synchronization::definitions::DefinitionStore {
        crate::synchronization::definitions::DefinitionStore::new(self.database.clone())
    }
    pub fn export_current_session(
        &self,
        id: &AgentSessionId,
        device_id: String,
    ) -> Result<SessionChange, String> {
        let sequence = self
            .database
            .read("read export sequence", |db| {
                db.query_row(
                    "SELECT COALESCE(MAX(sequence),0) FROM orchid_sync_changes WHERE session_id=?1",
                    [id.as_str()],
                    |r| r.get(0),
                )
            })
            .map_err(|e| e.to_string())?;
        self.export_session(id, device_id, sequence)
    }
    pub fn working_directory_for_folder(
        &self,
        folder: &crate::agent_sessions::organization::SessionFolderTarget,
    ) -> Result<String, String> {
        use crate::agent_sessions::organization::SessionFolderTarget;
        let repository = match folder {
            SessionFolderTarget::Repository { repository_id } => repository_id.clone(),
            SessionFolderTarget::WorkflowInstance { instance_id } => {
                crate::workflows::instances::WorkflowInstanceStore::from_database(
                    self.database.clone(),
                )
                .list_navigation()?
                .into_iter()
                .find(|i| i.id == *instance_id)
                .ok_or("Workflow instance not found on the execution device")?
                .repository_id
            }
        };
        Ok(
            crate::repository_catalog::RepositoryCatalog::new(self.database.clone())
                .resolve_main_working_tree(&repository)?
                .to_string_lossy()
                .into_owned(),
        )
    }
    pub fn export_session(
        &self,
        id: &AgentSessionId,
        device_id: String,
        sequence: u64,
    ) -> Result<SessionChange, String> {
        self.database.read("export consistent session snapshot", |connection| -> Result<SessionChange,String> {
        let tx=connection.unchecked_transaction().map_err(|e|e.to_string())?;
        let mut history=read_history_from(&tx,id,false).map_err(|e|e.to_string())?.ok_or("Session not found")?;
        if let Some(profile)=history.session.session_profile.as_mut(){*profile=profile.portable_copy().map_err(|e|e.to_string())?;}
        if let Some(target)=history.session.execution_target.as_mut(){if target.execution.device_id=="local"{target.execution.device_id=device_id.clone();target.execution.connection=crate::execution_targets::domain::ExecutionConnection::Device;}}
        let queue=queue::read_snapshot(&tx,id).map_err(|e|e.to_string())?;
        let mut preparations=Vec::new();
        let mut invocation_devices=std::collections::BTreeMap::new();
        for turn in &history.invocations {
            let origin:Option<String>=tx.query_row("SELECT device_id FROM orchid_invocation_devices WHERE invocation_id=?1",[turn.invocation.id.as_str()],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            invocation_devices.insert(turn.invocation.id.to_string(),origin.unwrap_or_else(||device_id.clone()));
            if let Some(mut preparation)=preparation::read_record(&tx,&turn.invocation.id).map_err(|e|e.to_string())? {
                preparation.current_resolution=preparation.current_resolution.map(|p|p.portable_copy().map_err(|e|e.to_string())).transpose()?;
                if let Some(snapshot)=preparation.execution_snapshot.as_mut(){snapshot.session_policy=snapshot.session_policy.portable_copy().map_err(|e|e.to_string())?;}
                preparations.push(preparation);
            }
        }
        let target_operations=tx.prepare("SELECT payload_json FROM agent_session_target_operations WHERE session_id=?1").map_err(|e|e.to_string())?
            .query_map([id.as_str()],|row|row.get::<_,String>(0)).map_err(|e|e.to_string())?
            .collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?
            .into_iter().map(|json|serde_json::from_str(&json).map_err(|e|e.to_string())).collect::<Result<Vec<_>,_>>()?;
        let prefix:Option<String>=tx.query_row("SELECT prefix_json FROM agent_session_initial_prompt_prefixes WHERE session_id=?1",[id.as_str()],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let initial_prompt_prefix=prefix.map(|json|serde_json::from_str(&json).map_err(|e|e.to_string())).transpose()?;
        let transfers=super::handoff::transfers_from(&tx,id).map_err(|e|e.to_string())?;
        let address=super::addressing::StoredAddressDomain::export(&tx,id)?;
        let provider_conversations=tx.prepare("SELECT binding_json FROM agent_session_provider_bindings WHERE session_id=?1 ORDER BY execution_key").map_err(|e|e.to_string())?.query_map([id.as_str()],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.map(|row|serde_json::from_str(&row.map_err(|e|e.to_string())?).map_err(|e|e.to_string())).collect::<Result<Vec<_>,_>>()?;
        let current:Option<String>=tx.query_row("SELECT resolution_json FROM agent_session_current_execution WHERE session_id=?1",[id.as_str()],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let current_profile=current.map(|json|serde_json::from_str::<crate::execution_configuration::SessionCreationResolution>(&json).map_err(|e|e.to_string())?.portable_copy().map_err(|e|e.to_string())).transpose()?;
        let fork=super::fork::read_fork(&tx,id)?;
        tx.commit().map_err(|e|e.to_string())?;
        let session=SessionReplica{history,queue,preparations,target_operations,initial_prompt_prefix,invocation_devices,current_profile,address,provider_conversations,fork};
        let mut portable=serde_json::to_value(session).map_err(|e|e.to_string())?;
        crate::synchronization::definitions::portable_routes(&mut portable,&device_id,false);
        Ok(SessionChange{device_id,sequence,transfers,session:serde_json::from_value(portable).map_err(|e|e.to_string())?})
        }).map_err(|e|e.to_string())
    }
}
