use super::*;
use std::{
    collections::VecDeque,
    io::{self, Read},
    sync::Condvar,
};
#[derive(Default)]
struct Wire {
    bytes: VecDeque<u8>,
    exited: bool,
}
#[derive(Default)]
struct FakeChild {
    wire: Mutex<Wire>,
    changed: Condvar,
    requests: Mutex<Vec<Value>>,
    block_initialize: bool,
}
impl FakeChild {
    fn output(&self, value: Value) {
        let mut wire = self.wire.lock().unwrap();
        wire.bytes.extend(value.to_string().bytes());
        wire.bytes.push_back(b'\n');
        self.changed.notify_all();
    }
    fn finish(&self) {
        self.output(json!({"method":"item/completed","params":{"threadId":"thread-one","turnId":"turn-one","item":{"id":"answer","type":"agentMessage","text":"STEERED"}}}));
        self.output(json!({"method":"turn/completed","params":{"threadId":"thread-one","turn":{"id":"turn-one","status":"completed"}}}));
    }
}
struct Reader(Arc<FakeChild>);
impl Read for Reader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut wire = self.0.wire.lock().unwrap();
        while wire.bytes.is_empty() && !wire.exited {
            wire = self.0.changed.wait(wire).unwrap();
        }
        let length = output.len().min(wire.bytes.len());
        for byte in &mut output[..length] {
            *byte = wire.bytes.pop_front().unwrap();
        }
        Ok(length)
    }
}
impl SupervisedChild for FakeChild {
    fn write_input(&self, bytes: &[u8]) -> io::Result<()> {
        let value: Value = serde_json::from_slice(bytes)?;
        self.requests.lock().unwrap().push(value.clone());
        let id = &value["id"];
        match value["method"].as_str() {
            Some("initialize") if self.block_initialize => {},
            Some("initialize") => self.output(json!({"id":id,"result":{}})),
            Some("thread/start" | "thread/resume") => self.output(json!({"id":id,"result":{"thread":{"id":"thread-one"},"cwd":value["params"]["cwd"],"model":"native-model","reasoningEffort":"high","approvalPolicy":"on-request","sandbox":{"type":"readOnly"}}})),
            Some("skills/list") => {
                let cwd = value["params"]["cwds"][0].as_str().unwrap();
                self.output(json!({"id":id,"result":{"data":[{"cwd":cwd,"skills":[{"name":"review","path":std::path::Path::new(cwd).join("SKILL.md"),"enabled":true}],"errors":[]}]}}));
            }
            Some("turn/start") => {
                self.output(json!({"id":id,"result":{"turn":{"id":"turn-one"}}}));
                self.output(json!({"method":"turn/started","params":{"threadId":"thread-one","turn":{"id":"turn-one","status":"inProgress"}}}));
                self.output(json!({"id":"approval-native","method":"item/commandExecution/requestApproval","params":{"threadId":"thread-one","turnId":"turn-one","itemId":"command-one","command":"echo approved","availableDecisions":["accept","decline"]}}));
            }
            Some("turn/steer") => {
                if value["params"]["input"][0]["text"] == "DISCONNECT" { return self.terminate(); }
                self.output(json!({"id":id,"result":{"turnId":"turn-one"}}));
                if value["params"]["input"][0]["text"] == "FINISH" { self.finish(); }
            }
            Some("turn/interrupt") => {
                self.output(json!({"id":id,"result":{}}));
                self.output(json!({"method":"turn/completed","params":{"threadId":"thread-one","turn":{"id":"turn-one","status":"interrupted"}}}));
            }
            _ => {}
        }
        Ok(())
    }
    fn close_input(&self) -> io::Result<()> {
        self.terminate()
    }
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        Ok(self.wire.lock().unwrap().exited.then_some(ProcessExit {
            exit_code: Some(0),
            signal: None,
        }))
    }
    fn terminate(&self) -> io::Result<()> {
        self.wire.lock().unwrap().exited = true;
        self.changed.notify_all();
        Ok(())
    }
    fn wait_after_termination(&self) -> io::Result<ProcessExit> {
        Ok(ProcessExit {
            exit_code: Some(0),
            signal: None,
        })
    }
}
#[derive(Default)]
struct Factory(Mutex<Vec<Arc<FakeChild>>>, bool);
impl ChildProcessFactory for Factory {
    fn spawn(&self, _: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        let child = Arc::new(FakeChild {
            block_initialize: self.1,
            ..Default::default()
        });
        self.0.lock().unwrap().push(child.clone());
        Ok(SpawnedProcess {
            child: child.clone(),
            stdout: Box::new(Reader(child)),
            stderr: Box::new(io::Cursor::new(Vec::<u8>::new())),
        })
    }
}

#[derive(Default)]
struct Sink(Mutex<Vec<RuntimeUpdate>>);
impl AgentRuntimeUpdateSink for Sink {
    fn emit_update(
        &self,
        _: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        self.0.lock().unwrap().push(update);
        Ok(())
    }
    fn report_delivery_failure(&self, _: &AgentInvocationId, _: RuntimeUpdateDeliveryFailure) {}
}
fn request(cwd: &std::path::Path) -> RuntimeInvocationRequest {
    RuntimeInvocationRequest {
        session_id: AgentSessionId::new("session").unwrap(),
        invocation_id: AgentInvocationId::new("invocation").unwrap(),
        submitted_text: "frozen original prompt".into(),
        working_directory: Some(cwd.to_string_lossy().into_owned()),
        options: Default::default(),
        launch_extension: None,
    }
}
#[test]
fn preparation_retains_native_identity_and_cwd_without_delivering_until_release() {
    let directory = tempfile::tempdir().unwrap();
    let factory = Arc::new(Factory::default());
    let runtime = CodexAppServerRuntime::new("fake", factory.clone());
    let request = request(directory.path());
    let id = request.invocation_id.clone();
    let ready = runtime
        .prepare_invocation(
            request,
            Some(ExternalRuntimeContextId::new("thread-one").unwrap()),
            Arc::new(Sink::default()),
        )
        .unwrap();
    assert_eq!(ready.external_context_id.as_str(), "thread-one");
    assert_eq!(ready.working_directory, directory.path().to_string_lossy());
    let child = factory.0.lock().unwrap()[0].clone();
    assert!(!child
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|r| r["method"] == "turn/start"));
    assert!(!child
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|r| r["method"] == "skills/extraRoots/set"));
    assert!(runtime.active_turn(&id).is_err());
    runtime.deliver_prepared_invocation(&id).unwrap();
    assert!(runtime.deliver_prepared_invocation(&id).is_err());
    let calls = child.requests.lock().unwrap();
    let delivered = calls
        .iter()
        .filter(|r| r["method"] == "turn/start")
        .collect::<Vec<_>>();
    assert_eq!(delivered.len(), 1);
    assert_eq!(
        delivered[0]["params"]["input"][0]["text"],
        "frozen original prompt"
    );
    drop(calls);
    runtime.shutdown().unwrap();
}

#[test]
fn preparation_does_not_force_invoke_selected_skills() {
    let directory = tempfile::tempdir().unwrap();
    let skill = directory.path().join("SKILL.md");
    std::fs::write(&skill, "---\nname: review\n---\n").unwrap();
    let factory = Arc::new(Factory::default());
    let runtime = CodexAppServerRuntime::new("fake", factory.clone());
    let mut request = request(directory.path());
    request.launch_extension = Some(RuntimeLaunchExtension {
        skill_inputs: vec![RuntimeSkillInput {
            id: skill.to_string_lossy().into_owned(),
            name: "review".into(),
            path: skill.to_string_lossy().into_owned(),
            content_sha256: "pinned-for-adapter-test".into(),
            description: "Review skill".into(),
        }],
        ..Default::default()
    });
    let id = request.invocation_id.clone();
    runtime
        .prepare_invocation(request, None, Arc::new(Sink::default()))
        .unwrap();
    runtime.deliver_prepared_invocation(&id).unwrap();
    let child = factory.0.lock().unwrap()[0].clone();
    let request = child
        .requests
        .lock()
        .unwrap()
        .iter()
        .find(|request| request["method"] == "turn/start")
        .cloned()
        .unwrap();
    assert_eq!(request["params"]["input"].as_array().unwrap().len(), 1);
    assert_eq!(request["params"]["input"][0]["type"], "text");
    assert_eq!(
        request["params"]["input"][0]["text"],
        "frozen original prompt"
    );
    runtime.shutdown().unwrap();
}

#[test]
fn mentioned_pinned_native_skill_is_an_explicit_turn_input() {
    let directory = tempfile::tempdir().unwrap();
    let skill = directory.path().join("SKILL.md");
    std::fs::write(&skill, "---\nname: review\n---\n").unwrap();
    let factory = Arc::new(Factory::default());
    let runtime = CodexAppServerRuntime::new("fake", factory.clone());
    let mut request = request(directory.path());
    request.submitted_text = "$review Examine this.".into();
    request.launch_extension = Some(RuntimeLaunchExtension {
        skill_inputs: vec![RuntimeSkillInput {
            id: skill.to_string_lossy().into_owned(),
            name: "review".into(),
            path: skill.to_string_lossy().into_owned(),
            content_sha256: "pinned-for-adapter-test".into(),
            description: String::new(),
        }],
        ..Default::default()
    });
    let id = request.invocation_id.clone();
    runtime
        .prepare_invocation(request, None, Arc::new(Sink::default()))
        .unwrap();
    runtime.deliver_prepared_invocation(&id).unwrap();
    let child = factory.0.lock().unwrap()[0].clone();
    let requests = child.requests.lock().unwrap();
    let turn = requests
        .iter()
        .find(|request| request["method"] == "turn/start")
        .unwrap();
    assert_eq!(turn["params"]["input"][0]["type"], "text");
    assert_eq!(turn["params"]["input"][1]["type"], "skill");
    assert_eq!(turn["params"]["input"][1]["name"], "review");
    assert_eq!(
        turn["params"]["input"][1]["path"],
        skill.to_string_lossy().as_ref()
    );
    drop(requests);
    runtime.shutdown().unwrap();
}
#[test]
fn cancel_during_native_initialization_never_delivers_a_prompt() {
    let directory = tempfile::tempdir().unwrap();
    let factory = Arc::new(Factory(Mutex::new(vec![]), true));
    let runtime = Arc::new(CodexAppServerRuntime::new("fake", factory.clone()));
    let request = request(directory.path());
    let id = request.invocation_id.clone();
    let worker_runtime = runtime.clone();
    let worker = std::thread::spawn(move || {
        worker_runtime.prepare_invocation(request, None, Arc::new(Sink::default()))
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if factory.0.lock().unwrap().first().is_some_and(|child| {
            child
                .requests
                .lock()
                .unwrap()
                .iter()
                .any(|r| r["method"] == "initialize")
        }) {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.cancel_invocation(&id).unwrap();
    assert!(worker.join().unwrap().is_err());
    let child = factory.0.lock().unwrap()[0].clone();
    assert!(!child
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|r| r["method"] == "turn/start"));
    assert!(runtime.deliver_prepared_invocation(&id).is_err());
    runtime.shutdown().unwrap();
}
