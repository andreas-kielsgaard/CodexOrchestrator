//! JSON-RPC framing and correlation. No Session persistence or provider semantics.
use crate::{
    contracts::{
        domain::AgentInvocationId,
        ports::{RuntimePortError, RuntimePortErrorKind},
    },
    processes::ProcessSupervisor,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

pub(super) struct Connection {
    pub(super) invocation_id: AgentInvocationId,
    supervisor: Arc<ProcessSupervisor>,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Value, RuntimePortError>>>>,
    buffer: Mutex<Vec<u8>>,
}

impl Connection {
    pub(super) fn new(
        invocation_id: AgentInvocationId,
        supervisor: Arc<ProcessSupervisor>,
    ) -> Self {
        Self {
            invocation_id,
            supervisor,
            next_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            buffer: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn write(&self, message: Value) -> Result<(), RuntimePortError> {
        let mut bytes = serde_json::to_vec(&message).map_err(|e| unavailable(e.to_string()))?;
        bytes.push(b'\n');
        self.supervisor
            .write_input(&self.invocation_id, &bytes)
            .map_err(|e| unavailable(e.to_string()))
    }

    pub(super) fn call(&self, method: &str, params: Value) -> Result<Value, RuntimePortError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|_| unavailable("RPC pending lock poisoned"))?
            .insert(id, tx);
        let sent = self.write(json!({"id":id,"method":method,"params":params}));
        let result = sent.and_then(|()| {
            rx.recv_timeout(Duration::from_secs(30)).map_err(|_| {
                unavailable(format!(
                    "No acknowledgement for {method}; delivery is uncertain"
                ))
            })?
        });
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&id);
        }
        result
    }

    pub(super) fn read(&self, bytes: &[u8]) -> Result<Vec<Value>, RuntimePortError> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|_| unavailable("RPC frame lock poisoned"))?;
        buffer.extend_from_slice(bytes);
        if buffer.len() > 32 * 1024 * 1024 {
            return Err(unavailable("App-server frame exceeds 32 MiB"));
        }
        let mut notifications = Vec::new();
        while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = buffer.drain(..=end).collect();
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let value: Value = serde_json::from_slice(&line)
                .map_err(|_| unavailable("Malformed app-server JSON-RPC frame"))?;
            if value.get("method").is_some() {
                notifications.push(value);
            } else if let Some(id) = value["id"].as_u64() {
                if let Some(tx) = self
                    .pending
                    .lock()
                    .map_err(|_| unavailable("RPC pending lock poisoned"))?
                    .remove(&id)
                {
                    let result = if value.get("error").is_some() {
                        Err(RuntimePortError::new(
                            RuntimePortErrorKind::UnsupportedOptions,
                            value["error"]["message"]
                                .as_str()
                                .unwrap_or("App-server rejected request"),
                        ))
                    } else {
                        Ok(value["result"].clone())
                    };
                    let _ = tx.send(result);
                }
            }
        }
        Ok(notifications)
    }

    pub(super) fn disconnected(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            for (_, sender) in pending.drain() {
                let _ = sender.send(Err(unavailable(
                    "App-server disconnected before acknowledgement; delivery is uncertain",
                )));
            }
        }
    }

    pub(super) fn finish_turn(&self) {
        let _ = self.supervisor.release_session(&self.invocation_id);
        let _ = self.supervisor.close_input(&self.invocation_id);
    }
}

pub(super) fn unavailable(message: impl Into<String>) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message)
}
