use crate::{
    otp_api::*,
    session_events::*,
    workflows::{
        compiled_plan::WorkflowSessionCreation,
        execution::{node_address, WorkflowExecutionService},
        instances::{
            RecipeInstance, SessionStopOutcome, WorkflowActionResult, WorkflowEventAttempt,
        },
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod delivery;
mod dispatch;
mod host;
mod routing;
pub use host::WorkflowHost;

fn reference(namespace: &str, kind: &str, id: &str) -> Result<ReferenceIdentity, String> {
    ReferenceIdentity::new(namespace, kind, id).map_err(|e| e.to_string())
}
