//! Newline-delimited JSON carried by an Orchid SSH stdio connection.
use crate::{
    configuration::{NativeCapabilityInventory, RuntimeProfileSnapshot},
    contracts::*,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostRequest {
    pub id: String,
    #[serde(flatten)]
    pub command: HostCommand,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HostCommand {
    Describe,
    PublishedCommit {
        repository_root: String,
        branch_ref: String,
    },
    MaterializeWorktree {
        repository_root: String,
        branch_ref: String,
        commit: String,
        instance_id: String,
    },
    AuxiliaryWorkspace {
        session_id: String,
    },
    ExportContinuation {
        configuration_ref: String,
        external_context_id: ExternalRuntimeContextId,
    },
    InstallContinuation {
        configuration_ref: String,
        continuation: crate::codex::app_server::continuation::CodexContinuation,
    },
    PrepareInvocation {
        configuration_ref: String,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
    },
    DeliverPreparedInvocation {
        invocation_id: AgentInvocationId,
    },
    ListWorktrees {
        repository_root: String,
        branch_ref: Option<String>,
    },
    Capabilities {
        configuration_ref: String,
        working_directory: Option<String>,
    },
    Preflight {
        configuration_ref: String,
        mode: RuntimeInvocationMode,
        options: AgentRuntimeOptions,
    },
    Invoke {
        configuration_ref: String,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
    },
    Respond {
        invocation_id: AgentInvocationId,
        request_id: String,
        response: Value,
    },
    Cancel {
        invocation_id: AgentInvocationId,
    },
    ActiveTurn {
        invocation_id: AgentInvocationId,
    },
    Steer {
        invocation_id: AgentInvocationId,
        target: RuntimeTurnTarget,
        input_id: String,
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HostFrame {
    Response {
        id: String,
        result: Option<Value>,
        error: Option<RuntimePortError>,
    },
    Update {
        invocation_id: AgentInvocationId,
        update: RuntimeUpdate,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDescription {
    pub device_id: String,
    pub device_name: String,
    pub configurations: Vec<HostConfigurationDescription>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConfigurationDescription {
    pub id: String,
    pub provider_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub profile: RuntimeProfileSnapshot,
    pub inventory: NativeCapabilityInventory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInstance {
    pub handle: String,
    pub path: String,
    pub branch_ref: Option<String>,
    pub head: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_preserve_request_and_invocation_identity() {
        let request: HostRequest = serde_json::from_str(
            r#"{"id":"request-1","method":"cancel","params":{"invocationId":"invocation-2"}}"#,
        )
        .unwrap();
        assert_eq!(request.id, "request-1");
        assert!(
            matches!(request.command, HostCommand::Cancel { invocation_id } if invocation_id.as_str() == "invocation-2")
        );
        let frame = HostFrame::Update {
            invocation_id: AgentInvocationId::new("invocation-2").unwrap(),
            update: RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Canceled,
                exit_code: None,
                signal: None,
                runtime_error: None,
            }),
        };
        let value = serde_json::to_value(&frame).unwrap();
        assert_eq!(value["kind"], "update");
        assert_eq!(value["invocationId"], "invocation-2");
        assert!(serde_json::from_value::<HostFrame>(value).is_ok());
    }
}
