//! Product navigation composes facts; Session execution stays independent of folders.
use super::{
    order::{validate_siblings, NavigationOrder, NavigationOrderScope},
    order_repository::NavigationOrderRepository,
};
use crate::{
    agent_sessions::{
        application::{
            configuration::SendDirectUserAgentSessionMessageResult, AgentSessionApplication,
        },
        domain::{AgentSessionAvailability, AgentSessionId},
        organization::{SessionFolderTarget, SessionOrganization, SessionPlacement},
        ports::{AgentSessionRepository, AgentSessionSummary, ListAgentSessionsQuery},
        repository::SqliteAgentSessionRepository,
    },
    repository_catalog::RepositoryCatalog,
    workflows::{
        instances::WorkflowInstanceStore,
        session_navigation::{project_session_owners, NavigationInstance, WorkflowSessionOwner},
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct NavigationRepository {
    pub id: String,
    pub name: String,
}
pub struct SessionNavigationData {
    pub branches: Vec<super::branch_inventory::NavigationBranch>,
    pub session_branches: Vec<super::branch_inventory::SessionBranch>,
    pub summaries: Vec<AgentSessionSummary>,
    pub repositories: Vec<NavigationRepository>,
    pub instances: Vec<NavigationInstance>,
    pub owners: Vec<WorkflowSessionOwner>,
    pub organization: Vec<SessionOrganization>,
    pub orders: Vec<NavigationOrder>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartSessionRequest {
    #[serde(default)]
    pub execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    pub submitted_text: String,
    #[serde(default)]
    pub attachments: Vec<crate::agent_sessions::domain::RuntimeImageAttachment>,
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub model: Option<String>,
    pub reasoning_mode: Option<String>,
    pub sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
    pub folder_target: Option<SessionFolderTarget>,
}
pub struct SessionNavigationService {
    pub branch_inventory: super::branch_inventory::BranchInventory,
    pub execution_targets: Arc<crate::execution_targets::ExecutionTargetService>,
    pub catalog: Arc<RepositoryCatalog>,
    instances: Arc<WorkflowInstanceStore>,
    repository: Arc<SqliteAgentSessionRepository>,
    sessions: Arc<AgentSessionApplication>,
    orders: NavigationOrderRepository,
}
impl SessionNavigationService {
    pub fn read_event_details(
        &self,
        session: &crate::agent_sessions::domain::AgentSessionId,
        id: &str,
    ) -> Result<crate::agent_sessions::domain::AgentRuntimeEvent, String> {
        self.repository
            .read_event_details(session, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Action details not found in this session".into())
    }
    pub fn new(
        catalog: Arc<RepositoryCatalog>,
        instances: Arc<WorkflowInstanceStore>,
        repository: Arc<SqliteAgentSessionRepository>,
        sessions: Arc<AgentSessionApplication>,
        orders: NavigationOrderRepository,
        execution_targets: Arc<crate::execution_targets::ExecutionTargetService>,
    ) -> Self {
        Self {
            branch_inventory: Default::default(),
            execution_targets,
            catalog,
            instances,
            repository,
            sessions,
            orders,
        }
    }
    pub fn load(&self) -> Result<SessionNavigationData, String> {
        let repositories = self
            .catalog
            .list_registered()?
            .into_iter()
            .map(|r| NavigationRepository {
                id: r.repository_id,
                name: r.label,
            })
            .collect();
        let instances = self.instances.list_navigation()?;
        let owners = project_session_owners(
            &instances,
            &self
                .repository
                .list_logical_addresses()
                .map_err(|e| e.to_string())?,
        )?;
        let summaries = self
            .repository
            .list_session_summaries(ListAgentSessionsQuery {
                availability: Some(AgentSessionAvailability::Available),
                limit: None,
            })
            .map_err(|e| e.to_string())?;
        let branches = self.branch_inventory.cached();
        let session_branches = super::branch_inventory::session_branches(&summaries, &branches);
        Ok(SessionNavigationData {
            branches,
            session_branches,
            orders: self.orders.load()?,
            repositories,
            instances,
            owners,
            summaries,
            organization: self
                .repository
                .list_organization()
                .map_err(|e| e.to_string())?,
        })
    }
    pub fn mark_attention(&self, id: AgentSessionId, unread: bool) -> Result<(), String> {
        self.repository
            .mark_attention(&id, unread)
            .map_err(|e| e.to_string())
    }
    pub fn reorder(
        &self,
        scope: NavigationOrderScope,
        ordered_ids: Vec<String>,
    ) -> Result<(), String> {
        let repositories = self.catalog.list_registered()?;
        let siblings: Vec<String> = match &scope {
            NavigationOrderScope::SessionLayout { folder_id } => {
                let data = self.load()?;
                let rows = super::session_order::session_ids(&data, folder_id);
                let unique: std::collections::HashSet<_> = ordered_ids.iter().collect();
                if unique.len() != ordered_ids.len()
                    || ordered_ids.iter().any(|id| {
                        !data.branches.iter().any(|branch| {
                            &branch.id == id
                                && format!("repo:{}:sessions", branch.repository_id) == *folder_id
                        }) && !id
                            .strip_prefix("session:")
                            .is_some_and(|id| rows.iter().any(|row| row == id))
                    })
                {
                    return Err("Invalid branch layout entry".into());
                }
                return self.orders.save(&scope, &ordered_ids);
            }
            NavigationOrderScope::Pinned => {
                let data = self.load()?;
                data.summaries
                    .iter()
                    .filter(|s| {
                        data.organization
                            .iter()
                            .any(|m| m.session_id == s.session.id && m.pinned_at.is_some())
                    })
                    .map(|s| s.session.id.as_str().to_owned())
                    .collect()
            }
            NavigationOrderScope::Sessions { folder_id } => {
                super::session_order::session_ids(&self.load()?, folder_id)
            }
            NavigationOrderScope::Repositories => repositories
                .iter()
                .map(|r| r.repository_id.clone())
                .collect(),
            NavigationOrderScope::Sections { repository_id }
            | NavigationOrderScope::Workflows { repository_id } => {
                if !repositories
                    .iter()
                    .any(|r| &r.repository_id == repository_id)
                {
                    return Err("Repository is not registered".into());
                }
                if matches!(scope, NavigationOrderScope::Sections { .. }) {
                    vec!["sessions".into(), "workflows".into()]
                } else {
                    self.instances
                        .list_navigation()?
                        .into_iter()
                        .filter(|i| &i.repository_id == repository_id)
                        .map(|i| i.id)
                        .collect()
                }
            }
        };
        validate_siblings(&ordered_ids, &siblings)?;
        self.orders.save(&scope, &ordered_ids)
    }
    fn repository_for_folder(&self, target: &SessionFolderTarget) -> Result<String, String> {
        let id = match target {
            SessionFolderTarget::Repository { repository_id } => repository_id.clone(),
            SessionFolderTarget::WorkflowInstance { instance_id } => {
                self.instances
                    .list_navigation()?
                    .into_iter()
                    .find(|instance| &instance.id == instance_id)
                    .ok_or("Workflow instance not found")?
                    .repository_id
            }
        };
        if !self
            .catalog
            .list_registered()?
            .iter()
            .any(|r| r.repository_id == id)
        {
            return Err("Repository is not registered".into());
        }
        Ok(id)
    }
    pub fn move_session(
        &self,
        id: AgentSessionId,
        placement: SessionPlacement,
        ordered_ids: Option<Vec<String>>,
    ) -> Result<(), String> {
        match &placement {
            SessionPlacement::Repository { repository_id } => {
                self.repository_for_folder(&SessionFolderTarget::Repository {
                    repository_id: repository_id.clone(),
                })?;
            }
            SessionPlacement::WorkflowInstance { instance_id } => {
                self.repository_for_folder(&SessionFolderTarget::WorkflowInstance {
                    instance_id: instance_id.clone(),
                })?;
            }
            _ => {}
        }
        if let Some(ids) = ordered_ids {
            let data = self.load()?;
            if !data.summaries.iter().any(|s| s.session.id == id) {
                return Err("Session not found".into());
            }
            let folder_id = super::session_order::folder_id(&data, &id, Some(&placement));
            let mut siblings = super::session_order::session_ids(&data, &folder_id);
            if !siblings.iter().any(|s| s == id.as_str()) {
                siblings.push(id.as_str().to_owned());
            }
            validate_siblings(&ids, &siblings)?;
            return self.orders.save_with_placement(
                &NavigationOrderScope::Sessions { folder_id },
                &ids,
                Some((&id, &placement)),
            );
        }
        self.repository
            .move_session(&id, &placement)
            .map_err(|e| e.to_string())
    }
    pub fn pin_session(&self, id: AgentSessionId, pinned: bool) -> Result<(), String> {
        self.repository
            .pin_session(&id, pinned, chrono::Utc::now())
            .map_err(|e| e.to_string())
    }
    pub fn working_directory_for_folder(
        &self,
        target: &SessionFolderTarget,
    ) -> Result<String, String> {
        let repository = self.repository_for_folder(target)?;
        Ok(self
            .catalog
            .resolve_main_working_tree(&repository)?
            .to_string_lossy()
            .into_owned())
    }
    pub fn load_quick_features(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
        folder_target: Option<&SessionFolderTarget>,
        execution_target: Option<&crate::execution_targets::domain::SessionExecutionTarget>,
    ) -> Result<crate::execution_configuration::RuntimeQuickFeatures, String> {
        self.load_quick_features_for_configuration(
            session_id,
            working_directory,
            folder_target,
            execution_target,
            None,
        )
    }
    pub fn load_quick_features_for_configuration(
        &self,
        session_id: Option<&AgentSessionId>,
        working_directory: Option<&str>,
        folder_target: Option<&SessionFolderTarget>,
        execution_target: Option<&crate::execution_targets::domain::SessionExecutionTarget>,
        configuration: Option<&orchid_engine::contracts::ProviderConfigurationRef>,
    ) -> Result<crate::execution_configuration::RuntimeQuickFeatures, String> {
        let folder_directory = if session_id.is_none() && execution_target.is_none() {
            folder_target
                .map(|target| self.working_directory_for_folder(target))
                .transpose()?
        } else {
            None
        };
        self.sessions.load_quick_features_for_configuration(
            session_id,
            folder_directory.as_deref().or(working_directory),
            execution_target,
            configuration,
        )
    }
    pub fn start_session(
        &self,
        mut request: StartSessionRequest,
    ) -> Result<SendDirectUserAgentSessionMessageResult, String> {
        if let Some(target) = request
            .folder_target
            .as_ref()
            .filter(|_| request.execution_target.is_none())
        {
            request.working_directory = Some(self.working_directory_for_folder(target)?);
        }
        self.sessions
            .start_direct_user_session_with_target_and_attachments(
                request.submitted_text,
                request.attachments,
                request.title,
                request.working_directory,
                request.model,
                request.reasoning_mode,
                request.sandbox_mode,
                request.execution_target,
                request.folder_target.map(Into::into),
            )
            .map_err(|e| e.to_string())
    }
}
