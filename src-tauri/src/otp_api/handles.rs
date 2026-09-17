use super::{InvocationContext, RoutingReceipt};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct NodeDefinition {
    pub id: String,
    pub name: String,
    pub initial_prompt: Option<String>,
    pub configuration: Value,
}
#[derive(Clone, Debug)]
pub(crate) struct NodeSession {
    pub id: String,
    pub running: bool,
    pub created_sequence: u64,
    pub last_addressed_sequence: Option<u64>,
    pub created_by_event: Option<String>,
    pub created_by_session: Option<String>,
}

/// The only product operations available to an OTP invocation.
pub(crate) trait OtpHost {
    fn node(&self, context: &InvocationContext, node_id: &str) -> Result<NodeDefinition, String>;
    fn sessions(
        &self,
        context: &InvocationContext,
        node_id: &str,
    ) -> Result<Vec<NodeSession>, String>;
    fn emit(
        &self,
        context: &InvocationContext,
        output: &str,
        payload: Value,
    ) -> Result<RoutingReceipt, String>;
}
