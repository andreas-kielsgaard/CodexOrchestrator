//! Orchid's own runtime control vocabulary: interaction requests and responses, steering, the
//! active turn, working-directory resolution and process exit. These records travel as
//! runtime-sourced events and are read only through this type. Provider raw payloads stay
//! diagnostic evidence and never drive product decisions.
//!
//! The encoding is the one already stored in event history, so older records decode unchanged.
use super::{
    domain::AgentRuntimeEventSource,
    runtime::{RuntimeEventDraft, RuntimeTurnTarget},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum RuntimeControlRecord {
    /// The provider started a steerable turn.
    #[serde(rename = "runtime_turn_active")]
    TurnActive { target: RuntimeTurnTarget },
    /// A provider request awaiting the user.
    #[serde(rename = "runtime_request_opened")]
    RequestOpened { request: super::interactions::RuntimeRequest },
    /// A provider request Orchid cannot answer; the provider has already declined it.
    #[serde(rename = "runtime_request_unsupported")]
    RequestUnsupported {
        request: super::interactions::RuntimeRequest,
        method: String,
    },
    #[serde(rename = "runtime_request_response")]
    RequestResponse {
        id: String,
        state: String,
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(rename = "session_steering_pending", rename_all = "camelCase")]
    SteeringPending {
        input_id: String,
        text: String,
        target: RuntimeTurnTarget,
    },
    #[serde(rename = "session_steering_result")]
    SteeringResult {
        id: String,
        state: String,
        #[serde(default)]
        message: Option<String>,
    },
    /// Continuing without a working directory recovered the provider's own context directory.
    #[serde(rename = "runtime_working_directory_resolved", rename_all = "camelCase")]
    WorkingDirectoryResolved { cwd: String, thread_id: String },
    /// The provider process ended. Emitted by the adapter that owns the process; its absence
    /// means the invocation's own terminal outcome is the process outcome.
    #[serde(rename = "runtime_process_exit", rename_all = "camelCase")]
    ProcessExit {
        status: ProcessExitStatus,
        exit_code: Option<i32>,
        signal: Option<String>,
        #[serde(default)]
        after_turn_completion: bool,
        /// Adapter diagnostic text.
        #[serde(default)]
        evidence: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessExitStatus {
    Completed,
    Failed,
    Canceled,
    Interrupted,
}

impl RuntimeControlRecord {
    /// Decodes a stored or emitted event; anything else, including provider evidence, is `None`.
    pub fn from_event(source: AgentRuntimeEventSource, raw_payload: &Value) -> Option<Self> {
        (source == AgentRuntimeEventSource::Runtime)
            .then(|| serde_json::from_value(raw_payload.clone()).ok())
            .flatten()
    }

    pub fn into_draft(self) -> RuntimeEventDraft {
        RuntimeEventDraft {
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: serde_json::to_value(self).expect("control records serialize"),
            normalized: None,
        }
    }

    /// Records whose facts must be kept even after the invocation reached a terminal state.
    pub fn is_durable_after_terminal(&self) -> bool {
        matches!(
            self,
            Self::SteeringResult { .. } | Self::RequestResponse { .. } | Self::ProcessExit { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_the_records_already_stored_in_history() {
        let stored = [
            json!({"kind":"runtime_turn_active","target":{"threadId":"t","turnId":"u"}}),
            json!({"kind":"runtime_request_opened","request":{"id":"approval","kind":"approval"}}),
            json!({"kind":"runtime_request_unsupported","request":{"id":"x"},"method":"item/tool/requestUserInput"}),
            json!({"kind":"runtime_request_response","id":"approval","state":"responding"}),
            json!({"kind":"runtime_request_response","id":"approval","state":"pending","message":"Invalid choice"}),
            json!({"kind":"session_steering_pending","inputId":"i","text":"steer","target":{"threadId":"t","turnId":"u"}}),
            json!({"kind":"session_steering_result","id":"i","state":"accepted","message":null}),
            json!({"kind":"runtime_working_directory_resolved","cwd":"C:/w","threadId":"t"}),
            json!({"kind":"runtime_process_exit","status":"completed","exitCode":0,"signal":null,"afterTurnCompletion":true,"evidence":"Exited"}),
        ];
        for payload in stored {
            let record = RuntimeControlRecord::from_event(AgentRuntimeEventSource::Runtime, &payload)
                .unwrap_or_else(|| panic!("{payload}"));
            let reencoded = record.into_draft().raw_payload;
            for (key, value) in payload.as_object().unwrap() {
                if key == "request" {
                    // Typed requests add their defaults; the stored fields stay as recorded.
                    for (field, stored) in value.as_object().unwrap() {
                        assert_eq!(&reencoded[key][field], stored, "{key}.{field} in {payload}");
                    }
                    continue;
                }
                assert_eq!(&reencoded[key], value, "{key} in {payload}");
            }
        }
    }

    #[test]
    fn provider_evidence_and_unknown_kinds_are_not_control_records() {
        let exit = json!({"kind":"runtime_process_exit","status":"completed","exitCode":0,"signal":null});
        assert!(RuntimeControlRecord::from_event(AgentRuntimeEventSource::Stdout, &exit).is_none());
        for payload in [
            json!({"kind":"runtime_transport","transport":"native"}),
            json!({"type":"item.completed","item":{"id":"1"}}),
        ] {
            assert!(RuntimeControlRecord::from_event(AgentRuntimeEventSource::Runtime, &payload).is_none());
        }
    }
}
