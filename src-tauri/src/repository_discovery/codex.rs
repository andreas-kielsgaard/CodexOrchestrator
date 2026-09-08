use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LINE_BYTES: usize = 1024 * 1024;
const MAX_PAGES_PER_STATE: usize = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodexWorkspaceCandidate {
    pub(crate) path: PathBuf,
}

#[derive(Clone, Default)]
pub(crate) struct CodexDirectoryDiscovery;

impl CodexDirectoryDiscovery {
    pub(crate) fn list(&self) -> Result<Vec<CodexWorkspaceCandidate>, String> {
        let mut session = CodexAppServerSession::start()?;
        session.list_directories()
    }
}

struct CodexAppServerSession {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: mpsc::Receiver<Result<Value, String>>,
    next_request_id: u64,
}

impl CodexAppServerSession {
    fn start() -> Result<Self, String> {
        let program = crate::runtime::codex::resolve_program("codex".into())?;
        let mut child = Command::new(program)
            .args(["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Codex CLI is unavailable.".to_string())?;
        let input = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| "Codex App Server input is unavailable.".to_string())?,
        );
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Codex App Server output is unavailable.".to_string())?;
        let (sender, output) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() > MAX_LINE_BYTES => {
                        let _ = sender.send(Err("Codex returned too much data.".into()));
                        break;
                    }
                    Ok(_) => {
                        let parsed = serde_json::from_str(&line)
                            .map_err(|_| "Codex returned an invalid response.".to_string());
                        if sender.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(Err("Codex App Server output failed.".into()));
                        break;
                    }
                }
            }
        });
        let mut session = Self {
            child,
            input,
            output,
            next_request_id: 1,
        };
        session.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codex-orchestrator",
                    "title": "Codex Orchestrator",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "experimentalApi": false,
                    "requestAttestation": false,
                    "optOutNotificationMethods": []
                }
            }),
        )?;
        session.notify("initialized")?;
        Ok(session)
    }

    fn list_directories(&mut self) -> Result<Vec<CodexWorkspaceCandidate>, String> {
        let mut paths = BTreeSet::new();
        for archived in [false, true] {
            let mut cursor: Option<String> = None;
            for _ in 0..MAX_PAGES_PER_STATE {
                let response = self.request(
                    "thread/list",
                    json!({
                        "cursor": cursor,
                        "limit": 100,
                        "sourceKinds": [
                            "cli", "vscode", "exec", "appServer", "subAgent",
                            "subAgentReview", "subAgentCompact", "subAgentThreadSpawn",
                            "subAgentOther", "unknown"
                        ],
                        "archived": archived,
                        "useStateDbOnly": true
                    }),
                )?;
                let data = response
                    .get("data")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "Codex returned an invalid task list.".to_string())?;
                for thread in data {
                    if let Some(cwd) = thread.get("cwd").and_then(Value::as_str) {
                        if !cwd.trim().is_empty() {
                            paths.insert(PathBuf::from(cwd));
                        }
                    }
                }
                cursor = response
                    .get("nextCursor")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if cursor.is_none() {
                    break;
                }
            }
        }
        Ok(paths
            .into_iter()
            .map(|path| CodexWorkspaceCandidate { path })
            .collect())
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.write(&json!({ "method": method, "id": id, "params": params }))?;
        for _ in 0..256 {
            let message = self
                .output
                .recv_timeout(RESPONSE_TIMEOUT)
                .map_err(|_| "Codex App Server did not respond in time.".to_string())??;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("Codex App Server rejected the request: {error}"));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex returned an incomplete response.".to_string());
        }
        Err("Codex returned too many unrelated messages.".into())
    }

    fn notify(&mut self, method: &str) -> Result<(), String> {
        self.write(&json!({ "method": method }))
    }

    fn write(&mut self, value: &Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.input, value)
            .map_err(|_| "Codex App Server request could not be encoded.".to_string())?;
        self.input
            .write_all(b"\n")
            .and_then(|_| self.input.flush())
            .map_err(|_| "Codex App Server request could not be sent.".to_string())
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for CodexAppServerSession {
    fn drop(&mut self) {
        self.stop();
    }
}
