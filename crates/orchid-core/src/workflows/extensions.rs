//! Presentation and route evidence for extension-owned flows. Execution stays with the extension.
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionNodeKind {
    Function,
    Capability,
    Agent,
    ExternalSessions,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionWorkflowNode {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ExtensionNodeKind,
    pub description: &'static str,
    pub x: i32,
    pub y: i32,
    pub prompt_keys: Vec<&'static str>,
    pub session_ids: Vec<String>,
    pub notifications: crate::agent_sessions::notifications::NodeNotifications,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionWorkflowConnection {
    pub id: &'static str,
    pub name: &'static str,
    pub source: &'static str,
    pub destination: &'static str,
    pub event: &'static str,
    pub condition: &'static str,
    pub payload: &'static str,
    pub enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRouteDestination {
    pub node_id: String,
    pub session_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRouteEvidence {
    pub source_node_id: String,
    pub connection_ids: Vec<String>,
    pub destinations: Vec<ExtensionRouteDestination>,
}
