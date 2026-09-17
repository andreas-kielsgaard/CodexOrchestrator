use super::{
    domain::{SidecarBindingRegistration, CONTROL_PROTOCOL_VERSION},
    proxy::{run_proxy_listener, ProxyBindings},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::SocketAddr,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, Weak,
    },
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const SIDECAR_ARGUMENT: &str = "--harness-engine-sidecar";
const LISTEN_PORT_PREFIX: &str = "--listen-port=";
const CONTROL_DEADLINE: Duration = Duration::from_secs(5);

pub(crate) trait HarnessSidecarClient: Send + Sync {
    fn register_binding(&self, registration: SidecarBindingRegistration) -> Result<String, String>;
    fn ensure_binding(&self, registration: SidecarBindingRegistration) -> Result<(), String>;
    fn retire_binding(&self, binding_id: &str) -> Result<(), String>;
    fn prepare_invocation(&self, binding_id: &str, invocation_id: &str) -> Result<(), String>;
    fn proxy_address(&self) -> Result<SocketAddr, String>;
    fn shutdown(&self) -> Result<(), String>;
}

pub(crate) struct ProcessHarnessSidecar {
    state: Mutex<SidecarState>,
    stopping: AtomicBool,
}

struct SidecarState {
    process: Option<SidecarProcess>,
    retained: BTreeMap<String, SidecarBindingRegistration>,
    listen_port: Option<u16>,
}

struct SidecarProcess {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: mpsc::Receiver<Result<String, String>>,
    address: SocketAddr,
}

impl ProcessHarnessSidecar {
    pub(crate) fn start_system() -> Result<Arc<Self>, String> {
        let sidecar = Arc::new(Self {
            state: Mutex::new(SidecarState {
                process: None,
                retained: BTreeMap::new(),
                listen_port: None,
            }),
            stopping: AtomicBool::new(false),
        });
        {
            let mut state = sidecar
                .state
                .lock()
                .map_err(|_| "Harness sidecar state is unavailable.".to_string())?;
            spawn_process(&mut state)?;
        }
        start_monitor(Arc::downgrade(&sidecar))?;
        Ok(sidecar)
    }

    fn with_running<T>(
        &self,
        operation: impl FnOnce(&mut SidecarProcess) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Harness sidecar state is unavailable.".to_string())?;
        ensure_running(&mut state)?;
        let result = operation(
            state
                .process
                .as_mut()
                .ok_or_else(|| "Harness sidecar is unavailable.".to_string())?,
        );
        if result.is_err() {
            stop_process(state.process.take());
        }
        result
    }

    fn monitor_once(&self) {
        if self.stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let exited = state
            .process
            .as_mut()
            .and_then(|process| process.child.try_wait().ok())
            .flatten()
            .is_some();
        if exited || state.process.is_none() {
            stop_process(state.process.take());
            if let Err(error) = spawn_process(&mut state) {
                eprintln!("Harness sidecar recovery failed: {error}");
            }
        }
    }

    fn finish_retirement(
        &self,
        binding_id: &str,
        retirement: Result<ControlResponse, String>,
    ) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "Harness sidecar state is unavailable.".to_string())?
            .retained
            .remove(binding_id);
        retirement.map(|_| ())
    }
}

impl Drop for ProcessHarnessSidecar {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Ok(state) = self.state.get_mut() {
            stop_process(state.process.take());
        }
    }
}

impl Drop for SidecarProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl HarnessSidecarClient for ProcessHarnessSidecar {
    fn register_binding(&self, registration: SidecarBindingRegistration) -> Result<String, String> {
        registration.verify_digest()?;
        let response = self.with_running(|process| {
            send_command(
                process,
                ControlCommandKind::RegisterBinding {
                    binding: registration.clone(),
                },
            )
        })?;
        let token = response
            .harness_token
            .ok_or_else(|| "Harness sidecar did not return a Harness token.".to_string())?;
        let mut retained = registration;
        retained.harness_token = Some(token.clone());
        self.state
            .lock()
            .map_err(|_| "Harness sidecar state is unavailable.".to_string())?
            .retained
            .insert(retained.binding_id.clone(), retained);
        Ok(token)
    }

    fn ensure_binding(&self, registration: SidecarBindingRegistration) -> Result<(), String> {
        let expected_token = registration
            .harness_token
            .clone()
            .ok_or_else(|| "Retained Harness binding has no Harness token.".to_string())?;
        let returned = self.register_binding(registration)?;
        if returned == expected_token {
            Ok(())
        } else {
            Err("Harness sidecar changed a retained Harness token.".to_string())
        }
    }

    fn retire_binding(&self, binding_id: &str) -> Result<(), String> {
        let retirement = self.with_running(|process| {
            send_command(
                process,
                ControlCommandKind::RetireBinding {
                    binding_id: binding_id.to_string(),
                },
            )
        });
        self.finish_retirement(binding_id, retirement)
    }

    fn prepare_invocation(&self, binding_id: &str, invocation_id: &str) -> Result<(), String> {
        self.with_running(|process| {
            send_command(
                process,
                ControlCommandKind::PrepareInvocation {
                    binding_id: binding_id.to_string(),
                    invocation_id: invocation_id.to_string(),
                },
            )
        })
        .map(|_| ())
    }

    fn proxy_address(&self) -> Result<SocketAddr, String> {
        self.with_running(|process| Ok(process.address))
    }

    fn shutdown(&self) -> Result<(), String> {
        self.stopping.store(true, Ordering::SeqCst);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Harness sidecar state is unavailable.".to_string())?;
        if let Some(process) = state.process.as_mut() {
            let _ = send_command(process, ControlCommandKind::Shutdown);
        }
        stop_process(state.process.take());
        Ok(())
    }
}

fn start_monitor(sidecar: Weak<ProcessHarnessSidecar>) -> Result<(), String> {
    thread::Builder::new()
        .name("harness-sidecar-monitor".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(250));
            let Some(sidecar) = sidecar.upgrade() else {
                break;
            };
            if sidecar.stopping.load(Ordering::SeqCst) {
                break;
            }
            sidecar.monitor_once();
        })
        .map(|_| ())
        .map_err(|error| format!("Unable to start Harness sidecar monitor: {error}"))
}

fn ensure_running(state: &mut SidecarState) -> Result<(), String> {
    let exited = state
        .process
        .as_mut()
        .and_then(|process| process.child.try_wait().ok())
        .flatten()
        .is_some();
    if exited || state.process.is_none() {
        stop_process(state.process.take());
        spawn_process(state)?;
    }
    Ok(())
}

fn spawn_process(state: &mut SidecarState) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Unable to locate Harness sidecar executable: {error}"))?;
    let mut command = Command::new(executable);
    command
        .arg(SIDECAR_ARGUMENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(port) = state.listen_port {
        command.arg(format!("{LISTEN_PORT_PREFIX}{port}"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Unable to start Harness sidecar: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Harness sidecar control input is unavailable.".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Harness sidecar control output is unavailable.".to_string())?;
    let stdout = match start_control_reader(stdout) {
        Ok(stdout) => stdout,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let mut process = SidecarProcess {
        child,
        stdin: BufWriter::new(stdin),
        stdout,
        address: "127.0.0.1:0".parse().expect("loopback address"),
    };
    let ready_line = receive_control_line(&process, "readiness")?;
    let ready = serde_json::from_str::<ReadyMessage>(&ready_line)
        .map_err(|error| format!("Harness sidecar readiness is invalid: {error}"))?;
    if ready.protocol != CONTROL_PROTOCOL_VERSION || ready.message_type != "ready" {
        return Err("Harness sidecar returned an unsupported readiness contract.".to_string());
    }
    process.address = ready
        .listen_address
        .parse()
        .map_err(|error| format!("Harness sidecar address is invalid: {error}"))?;
    if let Some(expected_port) = state.listen_port {
        if process.address.port() != expected_port {
            return Err("Harness sidecar recovery did not restore its proxy port.".to_string());
        }
    }
    state.listen_port = Some(process.address.port());
    for registration in state.retained.values() {
        let expected = registration
            .harness_token
            .as_deref()
            .ok_or_else(|| "Retained Harness binding has no token.".to_string())?;
        let response = send_command(
            &mut process,
            ControlCommandKind::RegisterBinding {
                binding: registration.clone(),
            },
        )?;
        if response.harness_token.as_deref() != Some(expected) {
            stop_process(Some(process));
            return Err("Harness sidecar recovery changed a retained token.".to_string());
        }
    }
    state.process = Some(process);
    Ok(())
}

fn stop_process(process: Option<SidecarProcess>) {
    if let Some(mut process) = process {
        if process.child.try_wait().ok().flatten().is_none() {
            let _ = process.child.kill();
        }
        let _ = process.child.wait();
    }
}

fn start_control_reader(
    stdout: ChildStdout,
) -> Result<mpsc::Receiver<Result<String, String>>, String> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("harness-sidecar-control-reader".to_string())
        .spawn(move || {
            let mut stdout = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match stdout.read_line(&mut line) {
                    Ok(0) => {
                        let _ = sender.send(Err(
                            "Harness sidecar exited before sending a control response.".to_string(),
                        ));
                        break;
                    }
                    Ok(_) => {
                        if sender.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(format!(
                            "Unable to read Harness sidecar control output: {error}"
                        )));
                        break;
                    }
                }
            }
        })
        .map_err(|error| format!("Unable to start Harness sidecar control reader: {error}"))?;
    Ok(receiver)
}

fn receive_control_line(process: &SidecarProcess, operation: &str) -> Result<String, String> {
    receive_control_line_with_deadline(process, operation, CONTROL_DEADLINE)
}

fn receive_control_line_with_deadline(
    process: &SidecarProcess,
    operation: &str,
    deadline: Duration,
) -> Result<String, String> {
    match process.stdout.recv_timeout(deadline) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "Harness sidecar {operation} exceeded its control deadline."
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(format!(
            "Harness sidecar exited before completing {operation}."
        )),
    }
}

fn send_command(
    process: &mut SidecarProcess,
    command: ControlCommandKind,
) -> Result<ControlResponse, String> {
    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let command = ControlCommand {
        protocol: CONTROL_PROTOCOL_VERSION.to_string(),
        request_id: request_id.clone(),
        command,
    };
    serde_json::to_writer(&mut process.stdin, &command)
        .map_err(|error| format!("Unable to encode Harness sidecar command: {error}"))?;
    process
        .stdin
        .write_all(b"\n")
        .and_then(|_| process.stdin.flush())
        .map_err(|error| format!("Unable to send Harness sidecar command: {error}"))?;
    let line = receive_control_line(process, "command")?;
    if line.is_empty() {
        return Err("Harness sidecar exited before responding.".to_string());
    }
    let response = serde_json::from_str::<ControlResponse>(&line)
        .map_err(|error| format!("Harness sidecar response is invalid: {error}"))?;
    if response.protocol != CONTROL_PROTOCOL_VERSION || response.request_id != request_id {
        return Err("Harness sidecar response correlation failed.".to_string());
    }
    if response.ok {
        Ok(response)
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "Harness sidecar rejected the command.".to_string()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlCommand {
    protocol: String,
    request_id: String,
    #[serde(flatten)]
    command: ControlCommandKind,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum ControlCommandKind {
    RegisterBinding {
        binding: SidecarBindingRegistration,
    },
    PrepareInvocation {
        binding_id: String,
        invocation_id: String,
    },
    RetireBinding {
        binding_id: String,
    },
    Shutdown,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlResponse {
    protocol: String,
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadyMessage {
    protocol: String,
    #[serde(rename = "type")]
    message_type: String,
    listen_address: String,
}

pub fn run_if_requested() -> bool {
    if !std::env::args().any(|argument| argument == SIDECAR_ARGUMENT) {
        return false;
    }
    if let Err(error) = run_process() {
        let _ = writeln!(io::stderr(), "Harness sidecar failed: {error}");
        std::process::exit(2);
    }
    true
}

fn run_process() -> Result<(), String> {
    let port = std::env::args().find_map(|argument| {
        argument
            .strip_prefix(LISTEN_PORT_PREFIX)
            .and_then(|value| value.parse::<u16>().ok())
    });
    let listener = std::net::TcpListener::bind(("127.0.0.1", port.unwrap_or(0)))
        .map_err(|error| format!("Unable to bind Harness proxy: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Unable to configure Harness proxy: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("Unable to identify Harness proxy: {error}"))?;
    let bindings = Arc::new(std::sync::RwLock::new(ProxyBindings::default()));
    let cancellation = CancellationToken::new();
    let proxy_bindings = bindings.clone();
    let proxy_cancel = cancellation.clone();
    let proxy = thread::Builder::new()
        .name("harness-mcp-proxy".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("Harness proxy runtime");
            runtime.block_on(async move {
                let listener =
                    tokio::net::TcpListener::from_std(listener).expect("Harness proxy listener");
                run_proxy_listener(listener, proxy_bindings, proxy_cancel).await;
            });
        })
        .map_err(|error| format!("Unable to start Harness proxy: {error}"))?;
    let stdout = io::stdout();
    let mut stdout = BufWriter::new(stdout.lock());
    serde_json::to_writer(
        &mut stdout,
        &ReadyMessage {
            protocol: CONTROL_PROTOCOL_VERSION.to_string(),
            message_type: "ready".to_string(),
            listen_address: address.to_string(),
        },
    )
    .map_err(|error| format!("Unable to encode Harness readiness: {error}"))?;
    stdout
        .write_all(b"\n")
        .and_then(|_| stdout.flush())
        .map_err(|error| format!("Unable to publish Harness readiness: {error}"))?;

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line =
            line.map_err(|error| format!("Unable to read Harness control input: {error}"))?;
        let value = serde_json::from_str::<serde_json::Value>(&line)
            .map_err(|error| format!("Harness control command is invalid: {error}"))?;
        let protocol = value
            .get("protocol")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let request_id = value
            .get("requestId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let command_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let mut response = ControlResponse {
            protocol: CONTROL_PROTOCOL_VERSION.to_string(),
            request_id,
            ok: false,
            harness_token: None,
            error: None,
        };
        if protocol != CONTROL_PROTOCOL_VERSION {
            response.error = Some("Unsupported Harness control protocol.".to_string());
        } else {
            match command_type {
                "register_binding" => {
                    let result = value
                        .get("binding")
                        .cloned()
                        .ok_or_else(|| "Harness registration is missing a binding.".to_string())
                        .and_then(|binding| {
                            serde_json::from_value::<SidecarBindingRegistration>(binding).map_err(
                                |error| format!("Harness registration is invalid: {error}"),
                            )
                        })
                        .and_then(|binding| {
                            bindings
                                .write()
                                .map_err(|_| {
                                    "Harness proxy binding state is unavailable.".to_string()
                                })?
                                .register(binding)
                        });
                    match result {
                        Ok(token) => {
                            response.ok = true;
                            response.harness_token = Some(token);
                        }
                        Err(error) => response.error = Some(error),
                    }
                }
                "retire_binding" => {
                    if let Some(binding_id) =
                        value.get("bindingId").and_then(serde_json::Value::as_str)
                    {
                        if let Ok(mut bindings) = bindings.write() {
                            bindings.retire(binding_id);
                            response.ok = true;
                        } else {
                            response.error =
                                Some("Harness proxy binding state is unavailable.".to_string());
                        }
                    } else {
                        response.error =
                            Some("Harness retirement is missing a binding ID.".to_string());
                    }
                }
                "prepare_invocation" => {
                    let binding_id = value.get("bindingId").and_then(serde_json::Value::as_str);
                    let invocation_id = value
                        .get("invocationId")
                        .and_then(serde_json::Value::as_str);
                    match (binding_id, invocation_id) {
                        (Some(binding_id), Some(invocation_id)) => match bindings.write() {
                            Ok(mut bindings) => {
                                match bindings.prepare_invocation(binding_id, invocation_id) {
                                    Ok(()) => response.ok = true,
                                    Err(error) => response.error = Some(error),
                                }
                            }
                            Err(_) => {
                                response.error =
                                    Some("Harness proxy binding state is unavailable.".to_string())
                            }
                        },
                        _ => {
                            response.error =
                                Some("Harness invocation preparation is incomplete.".to_string())
                        }
                    }
                }
                "shutdown" => {
                    response.ok = true;
                }
                _ => response.error = Some("Unknown Harness control command.".to_string()),
            }
        }
        serde_json::to_writer(&mut stdout, &response)
            .map_err(|error| format!("Unable to encode Harness control response: {error}"))?;
        stdout
            .write_all(b"\n")
            .and_then(|_| stdout.flush())
            .map_err(|error| format!("Unable to publish Harness control response: {error}"))?;
        if command_type == "shutdown" && response.ok {
            break;
        }
    }
    cancellation.cancel();
    let _ = proxy.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::domain::{
        binding_digest, HarnessMediationPlan, MEDIATION_PLAN_VERSION,
    };

    fn registration(token: Option<&str>) -> SidecarBindingRegistration {
        let snapshot = "{\"harnessName\":\"Review\"}".to_string();
        let plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.to_string(),
            exposures: Vec::new(),
        })
        .unwrap();
        SidecarBindingRegistration {
            binding_id: "binding-1".into(),
            session_id: "session-1".into(),
            runtime_instance_id: "runtime-1".into(),
            session_instance_token: "session-token".into(),
            configuration_digest: binding_digest(&snapshot, &plan),
            harness_snapshot: snapshot,
            mediation_plan: plan,
            harness_token: token.map(str::to_string),
            source_workflow_instance_id: "workflow-instance-1".into(),
            source_node_id: "node-1".into(),
        }
    }

    #[test]
    fn restart_registration_restores_exact_token_and_bytes() {
        let mut first = ProxyBindings::default();
        let token = first.register(registration(None)).expect("initial binding");
        let mut restored = ProxyBindings::default();
        let restored_token = restored
            .register(registration(Some(&token)))
            .expect("restored binding");
        assert_eq!(restored_token, token);
        assert_eq!(
            restored.by_token.get(&token).unwrap().registration,
            registration(Some(&token))
        );
    }

    #[test]
    fn control_deadline_returns_an_error_and_terminates_the_owned_child() {
        #[cfg(windows)]
        let mut child = {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
            command
        };
        #[cfg(not(windows))]
        let mut child = {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 30"]);
            command
        };
        let mut child = child
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start silent owned child");
        let stdin = child.stdin.take().unwrap();
        let stdout = start_control_reader(child.stdout.take().unwrap()).unwrap();
        let sidecar = ProcessHarnessSidecar {
            state: Mutex::new(SidecarState {
                process: Some(SidecarProcess {
                    child,
                    stdin: BufWriter::new(stdin),
                    stdout,
                    address: "127.0.0.1:43123".parse().unwrap(),
                }),
                retained: BTreeMap::new(),
                listen_port: Some(43123),
            }),
            stopping: AtomicBool::new(false),
        };

        let error = sidecar
            .with_running(|process| {
                receive_control_line_with_deadline(
                    process,
                    "test command",
                    Duration::from_millis(20),
                )
            })
            .unwrap_err();

        assert!(error.contains("control deadline"));
        assert!(sidecar.state.lock().unwrap().process.is_none());
    }

    #[test]
    fn failed_retirement_forgets_the_local_token_before_prepared_retry() {
        let retained = registration(Some("retained-token"));
        let sidecar = ProcessHarnessSidecar {
            state: Mutex::new(SidecarState {
                process: None,
                retained: BTreeMap::from([(retained.binding_id.clone(), retained)]),
                listen_port: Some(43123),
            }),
            stopping: AtomicBool::new(false),
        };

        let error = sidecar
            .finish_retirement("binding-1", Err("retirement timed out".to_string()))
            .unwrap_err();

        assert!(error.contains("timed out"));
        assert!(sidecar.state.lock().unwrap().retained.is_empty());
    }
}
