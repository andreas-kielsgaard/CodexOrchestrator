//! Display placement is independent of a Session's execution context and logical address.
use super::domain::AgentSessionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SessionFolderTarget {
    Repository {
        #[serde(rename = "repositoryId")]
        repository_id: String,
    },
    WorkflowInstance {
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SessionPlacement {
    #[default]
    Default,
    Unfiled,
    Repository {
        #[serde(rename = "repositoryId")]
        repository_id: String,
    },
    WorkflowInstance {
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
}
impl From<SessionFolderTarget> for SessionPlacement {
    fn from(target: SessionFolderTarget) -> Self {
        match target {
            SessionFolderTarget::Repository { repository_id } => Self::Repository { repository_id },
            SessionFolderTarget::WorkflowInstance { instance_id } => {
                Self::WorkflowInstance { instance_id }
            }
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionOrganization {
    pub(crate) session_id: AgentSessionId,
    pub(crate) placement: SessionPlacement,
    pub(crate) pinned_at: Option<DateTime<Utc>>,
}
