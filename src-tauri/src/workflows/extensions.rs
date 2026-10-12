//! Presentation and route evidence for extension-owned flows. Execution stays with the extension.
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExtensionNodeKind {
    Function,
    Capability,
    Agent,
    ExternalSessions,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionWorkflowNode {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: ExtensionNodeKind,
    pub(crate) description: &'static str,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) prompt_keys: Vec<&'static str>,
    pub(crate) session_ids: Vec<String>,
    pub(crate) notifications: crate::agent_sessions::notifications::NodeNotifications,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionWorkflowConnection {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) source: &'static str,
    pub(crate) destination: &'static str,
    pub(crate) event: &'static str,
    pub(crate) condition: &'static str,
    pub(crate) payload: &'static str,
    pub(crate) enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionRouteDestination {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionRouteEvidence {
    pub(crate) source_node_id: String,
    pub(crate) connection_ids: Vec<String>,
    pub(crate) destinations: Vec<ExtensionRouteDestination>,
}
