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
pub(crate) struct NavigationRepository {
    pub(crate) id: String,
    pub(crate) name: String,
}
pub(crate) struct SessionNavigationData {
    pub(crate) summaries: Vec<AgentSessionSummary>,
    pub(crate) repositories: Vec<NavigationRepository>,
    pub(crate) instances: Vec<NavigationInstance>,
    pub(crate) owners: Vec<WorkflowSessionOwner>,
    pub(crate) organization: Vec<SessionOrganization>,
    pub(crate) orders: Vec<NavigationOrder>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartSessionRequest {
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
    pub(crate) sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
    pub(crate) folder_target: Option<SessionFolderTarget>,
}
pub(crate) struct SessionNavigationService {
    catalog: Arc<RepositoryCatalog>,
    instances: Arc<WorkflowInstanceStore>,
    repository: Arc<SqliteAgentSessionRepository>,
    sessions: Arc<AgentSessionApplication>,
    orders: NavigationOrderRepository,
}
impl SessionNavigationService {
    pub(crate) fn new(
        catalog: Arc<RepositoryCatalog>,
        instances: Arc<WorkflowInstanceStore>,
        repository: Arc<SqliteAgentSessionRepository>,
        sessions: Arc<AgentSessionApplication>,
        orders: NavigationOrderRepository,
    ) -> Self {
        Self {
            catalog,
            instances,
            repository,
            sessions,
            orders,
        }
    }
    pub(crate) fn load(&self) -> Result<SessionNavigationData, String> {
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
        Ok(SessionNavigationData {
            orders: self.orders.load()?,
            repositories,
            instances,
            owners,
            summaries: self
                .repository
                .list_session_summaries(ListAgentSessionsQuery {
                    availability: Some(AgentSessionAvailability::Available),
                    limit: None,
                })
                .map_err(|e| e.to_string())?,
            organization: self
                .repository
                .list_organization()
                .map_err(|e| e.to_string())?,
        })
    }
    pub(crate) fn reorder(
        &self,
        scope: NavigationOrderScope,
        ordered_ids: Vec<String>,
    ) -> Result<(), String> {
        let repositories = self.catalog.list_registered()?;
        let siblings: Vec<String> = match &scope {
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
    pub(crate) fn move_session(
        &self,
        id: AgentSessionId,
        placement: SessionPlacement,
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
        self.repository
            .move_session(&id, &placement)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn pin_session(&self, id: AgentSessionId, pinned: bool) -> Result<(), String> {
        self.repository
            .pin_session(&id, pinned, chrono::Utc::now())
            .map_err(|e| e.to_string())
    }
    pub(crate) fn start_session(
        &self,
        mut request: StartSessionRequest,
    ) -> Result<SendDirectUserAgentSessionMessageResult, String> {
        if let Some(target) = &request.folder_target {
            let repository = self.repository_for_folder(target)?;
            request.working_directory = Some(
                self.catalog
                    .resolve_main_working_tree(&repository)?
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        self.sessions
            .start_direct_user_session(
                request.submitted_text,
                request.title,
                request.working_directory,
                request.model,
                request.reasoning_mode,
                request.sandbox_mode,
                request.folder_target.map(Into::into),
            )
            .map_err(|e| e.to_string())
    }
}
