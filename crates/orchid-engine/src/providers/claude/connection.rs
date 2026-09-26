//! JSON lines to and from one supervised `claude` process. Claude handles its input in order, so
//! Orchid never waits on its own control requests; their responses are dropped here.
use crate::{
    contracts::{
        domain::AgentInvocationId,
        ports::{RuntimePortError, RuntimePortErrorKind},
    },
    processes::{json_lines::JsonLines, ProcessSupervisor},
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub(super) struct Connection {
    pub(super) invocation_id: AgentInvocationId,
    supervisor: Arc<ProcessSupervisor>,
    next_request: AtomicU64,
    lines: Mutex<JsonLines>,
}

impl Connection {
    pub(super) fn new(
        invocation_id: AgentInvocationId,
        supervisor: Arc<ProcessSupervisor>,
    ) -> Self {
        Self {
            invocation_id,
            supervisor,
            next_request: AtomicU64::new(1),
            lines: Mutex::new(JsonLines::default()),
        }
    }

    pub(super) fn write(&self, message: Value) -> Result<(), RuntimePortError> {
        let mut bytes = serde_json::to_vec(&message).map_err(|e| unavailable(e.to_string()))?;
        bytes.push(b'\n');
        self.supervisor
            .write_input(&self.invocation_id, &bytes)
            .map_err(|e| unavailable(e.to_string()))
    }

    /// Sends one of Orchid's control requests, such as `initialize` or `interrupt`.
    pub(super) fn request(&self, subtype: &str) -> Result<(), RuntimePortError> {
        let id = format!(
            "orchid-{}",
            self.next_request.fetch_add(1, Ordering::Relaxed)
        );
        self.write(json!({"type":"control_request","request_id":id,"request":{"subtype":subtype}}))
    }

    /// Complete messages from a stdout chunk. Responses to Orchid's control requests are dropped:
    /// nothing waits on them, and `initialize` answers with account details.
    pub(super) fn read(&self, bytes: &[u8]) -> Result<Vec<Value>, RuntimePortError> {
        let values = self
            .lines
            .lock()
            .map_err(|_| unavailable("Output lock poisoned"))?
            .push(bytes)
            .map_err(|error| unavailable(format!("Claude output: {error}")))?;
        Ok(values
            .into_iter()
            .filter(|value| value["type"] != "control_response")
            .collect())
    }

    /// Ends the conversation: Claude exits once its input closes.
    pub(super) fn finish(&self) {
        let _ = self.supervisor.release_session(&self.invocation_id);
        let _ = self.supervisor.close_input(&self.invocation_id);
    }
}

pub(super) fn unavailable(message: impl Into<String>) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message)
}
