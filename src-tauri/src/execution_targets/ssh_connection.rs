//! SSH owns bytes and request correlation; the remote runtime owns session semantics.
use crate::agent_sessions::{domain::*, ports::*};
use orchid_engine::protocol::{HostCommand, HostFrame, HostRequest};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

type Reply = Result<Value, RuntimePortError>;
enum Delivery {
    Update(AgentInvocationId, RuntimeUpdate),
    Disconnected(String),
}
pub(crate) struct SshConnection {
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Child>,
    pending: Arc<Mutex<HashMap<String, mpsc::Sender<Reply>>>>,
    sinks: Arc<Mutex<HashMap<AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>>>>,
}

fn unavailable(message: impl Into<String>) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message)
}

impl SshConnection {
    pub(crate) fn connect(target: &str, executable: &str) -> Result<Self, RuntimePortError> {
        let mut command = Command::new("ssh");
        command.args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=10",
            target,
        ]);
        command.arg(format!("'{}' connect", executable.replace('\'', "'\\''")));
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| unavailable(format!("Unable to start SSH: {e}")))?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let diagnostics = Arc::new(Mutex::new(String::new()));
        let error_text = diagnostics.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Ok(mut text) = error_text.lock() {
                    if text.len() < 16000 {
                        text.push_str(&line);
                        text.push('\n');
                    }
                }
            }
        });
        let pending: Arc<Mutex<HashMap<String, mpsc::Sender<Reply>>>> = Default::default();
        let sinks: Arc<Mutex<HashMap<AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>>>> =
            Default::default();
        let response_channels = pending.clone();
        let event_sinks = sinks.clone();
        let (delivery_tx, delivery_rx) = mpsc::channel::<Delivery>();
        std::thread::spawn(move || {
            for delivery in delivery_rx {
                match delivery {
                    Delivery::Update(invocation_id, update) => {
                        let terminal = matches!(&update, RuntimeUpdate::Finished(_));
                        let sink = event_sinks.lock().unwrap().get(&invocation_id).cloned();
                        if let Some(sink) = sink {
                            if let Err(error) = sink.emit_update(&invocation_id, update.clone()) {
                                sink.report_delivery_failure(
                                    &invocation_id,
                                    RuntimeUpdateDeliveryFailure { update, error },
                                );
                            }
                        }
                        if terminal {
                            event_sinks.lock().unwrap().remove(&invocation_id);
                        }
                    }
                    Delivery::Disconnected(message) => {
                        let active = event_sinks.lock().unwrap().drain().collect::<Vec<_>>();
                        for (id, sink) in active {
                            let update = RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                                status: AgentInvocationTerminalStatus::Interrupted,
                                exit_code: None,
                                signal: None,
                                runtime_error: Some(AgentRuntimeFailure {
                                    code: "remote_disconnected".into(),
                                    message: message.clone(),
                                    details: None,
                                }),
                            });
                            if let Err(error) = sink.emit_update(&id, update.clone()) {
                                sink.report_delivery_failure(
                                    &id,
                                    RuntimeUpdateDeliveryFailure { update, error },
                                );
                            }
                        }
                        break;
                    }
                }
            }
        });
        std::thread::spawn(move || {
            let mut detail = String::new();
            for line in BufReader::new(stdout).lines() {
                let frame = match line
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str::<HostFrame>(&s).map_err(|e| e.to_string()))
                {
                    Ok(frame) => frame,
                    Err(error) => {
                        detail = error;
                        break;
                    }
                };
                match frame {
                    HostFrame::Response { id, result, error } => {
                        if let Some(tx) = response_channels.lock().unwrap().remove(&id) {
                            let _ = tx
                                .send(error.map_or_else(|| Ok(result.unwrap_or(Value::Null)), Err));
                        }
                    }
                    HostFrame::Update {
                        invocation_id,
                        update,
                    } => {
                        let _ = delivery_tx.send(Delivery::Update(invocation_id, update));
                    }
                }
            }
            let diagnostic = diagnostics.lock().unwrap().clone();
            let message = format!(
                "Remote connection closed; remote outcome is unconfirmed. {detail} {diagnostic}"
            );
            for (_, tx) in response_channels.lock().unwrap().drain() {
                let _ = tx.send(Err(unavailable(&message)));
            }
            let _ = delivery_tx.send(Delivery::Disconnected(message));
        });
        Ok(Self {
            stdin: Mutex::new(Some(stdin)),
            child: Mutex::new(child),
            pending,
            sinks,
        })
    }

    pub(crate) fn request<T: DeserializeOwned>(
        &self,
        command: HostCommand,
    ) -> Result<T, RuntimePortError> {
        if self
            .child
            .lock()
            .unwrap()
            .try_wait()
            .map_err(|e| unavailable(e.to_string()))?
            .is_some()
        {
            return Err(unavailable("Remote connection has closed"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);
        let bytes = serde_json::to_vec(&HostRequest {
            id: id.clone(),
            command,
        })
        .map_err(|e| unavailable(e.to_string()))?;
        let write = (|| {
            let mut input = self.stdin.lock().unwrap();
            let input = input
                .as_mut()
                .ok_or_else(|| unavailable("Remote connection is closed"))?;
            input
                .write_all(&bytes)
                .and_then(|_| input.write_all(b"\n"))
                .and_then(|_| input.flush())
                .map_err(|e| unavailable(e.to_string()))
        })();
        if let Err(error) = write {
            self.pending.lock().unwrap().remove(&id);
            return Err(error);
        }
        let result = rx
            .recv_timeout(Duration::from_secs(120))
            .map_err(|e| unavailable(format!("Remote host did not answer: {e}")));
        self.pending.lock().unwrap().remove(&id);
        serde_json::from_value(result??)
            .map_err(|e| unavailable(format!("Invalid remote reply: {e}")))
    }

    pub(crate) fn register(&self, id: AgentInvocationId, sink: Arc<dyn AgentRuntimeUpdateSink>) {
        self.sinks.lock().unwrap().insert(id, sink);
    }
    pub(crate) fn unregister(&self, id: &AgentInvocationId) {
        self.sinks.lock().unwrap().remove(id);
    }
    pub(crate) fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.stdin.lock().unwrap().take();
        let mut child = self.child.lock().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if child
                .try_wait()
                .map_err(|e| unavailable(e.to_string()))?
                .is_some()
            {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        child
            .kill()
            .and_then(|_| child.wait())
            .map(|_| ())
            .map_err(|e| unavailable(e.to_string()))
    }
}

impl Drop for SshConnection {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
