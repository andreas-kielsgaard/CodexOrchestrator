use super::*;
use crate::agent_sessions::{application::*, repository::SqliteAgentSessionRepository};
use crate::agent_sessions::ports::*;
use crate::runtime::processes::*;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::{self, Read},
    sync::Condvar,
    time::Instant,
};
use std::{
    sync::{Arc, Mutex, Weak},
    thread,
    time::Duration,
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
            Some("initialize") => self.output(json!({"id":id,"result":{}})),
            Some("thread/start" | "thread/resume") => self.output(json!({"id":id,"result":{"thread":{"id":"thread-one"},"model":"native-model","reasoningEffort":"high","approvalPolicy":"on-request","sandbox":{"type":"readOnly"}}})),
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

#[test]
fn disconnected_steering_stays_uncertain_and_is_never_replayed() {
    let folder = tempfile::tempdir().unwrap();
    let repository = Arc::new(
        SqliteAgentSessionRepository::new(
            crate::storage::open_active_database(&folder.path().join("sessions.sqlite")).unwrap(),
        )
        .unwrap(),
    );
    let factory = Arc::new(Factory::default());
    let runtime = Arc::new(CodexAppServerRuntime::new("fake-codex", factory.clone()));
    let providers = Arc::new(SystemAgentSessionProviders);
    let application = AgentSessionApplication::new(
        repository,
        runtime.clone(),
        Arc::new(Notifier::default()),
        providers.clone(),
        providers,
        None,
    )
    .with_workspaces(
        SessionWorkspaces::new(folder.path().join("product"), "database".into()).unwrap(),
    );
    let started = application
        .send_message(SendAgentSessionMessageCommand {
            session_id: None,
            submitted_text: "Wait".into(),
            title: None,
            working_directory: None,
            requested_options: None,
        })
        .unwrap();
    wait_until(|| runtime.active_turn(&started.invocation_id).is_ok());
    let command = SteerAgentSessionCommand {
        session_id: started.session_id.clone(),
        invocation_id: started.invocation_id.clone(),
        input_id: "lost-ack".into(),
        text: "DISCONNECT".into(),
    };
    assert_eq!(
        application.steer_session(command.clone()).unwrap().state,
        "uncertain"
    );
    wait_until(|| {
        application
            .load_session(&started.session_id)
            .unwrap()
            .invocations[0]
            .invocation
            .status
            .is_terminal()
    });
    assert_eq!(
        application.steer_session(command.clone()).unwrap().state,
        "uncertain"
    );
    assert!(application
        .steer_session(SteerAgentSessionCommand {
            text: "different".into(),
            ..command
        })
        .is_err());
    let child = factory.0.lock().unwrap()[0].clone();
    assert_eq!(
        child
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r["method"] == "turn/steer")
            .count(),
        1
    );
    runtime.shutdown().unwrap();
}

#[derive(Default)]
struct ReentrantNotifier {
    application: Mutex<Option<Weak<AgentSessionApplication>>>,
    successor: Mutex<Option<Result<SendAgentSessionMessageResult, String>>>,
}
impl AgentSessionNotifier for ReentrantNotifier {
    fn notify(&self, event: AgentSessionNotification) -> Result<(), String> {
        if let AgentSessionNotification::InvocationTerminal { session_id, .. } = event {
            let mut successor = self.successor.lock().unwrap();
            if successor.is_none() {
                let application = self
                    .application
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap();
                *successor = Some(
                    application
                        .send_message(SendAgentSessionMessageCommand {
                            session_id: Some(session_id),
                            submitted_text: "Successor".into(),
                            title: None,
                            working_directory: None,
                            requested_options: None,
                        })
                        .map_err(|e| e.to_string()),
                );
            }
        }
        Ok(())
    }
}

#[test]
fn terminal_observer_can_start_successor_before_old_process_exits() {
    let folder = tempfile::tempdir().unwrap();
    let repository = Arc::new(
        SqliteAgentSessionRepository::new(
            crate::storage::open_active_database(&folder.path().join("sessions.sqlite")).unwrap(),
        )
        .unwrap(),
    );
    let runtime = Arc::new(CodexAppServerRuntime::new(
        "fake-codex",
        Arc::new(Factory::default()),
    ));
    let notifier = Arc::new(ReentrantNotifier::default());
    let providers = Arc::new(SystemAgentSessionProviders);
    let application = Arc::new(
        AgentSessionApplication::new(
            repository,
            runtime.clone(),
            notifier.clone(),
            providers.clone(),
            providers,
            None,
        )
        .with_workspaces(
            SessionWorkspaces::new(folder.path().join("product"), "database".into()).unwrap(),
        ),
    );
    *notifier.application.lock().unwrap() = Some(Arc::downgrade(&application));
    let started = application
        .send_message(SendAgentSessionMessageCommand {
            session_id: None,
            submitted_text: "First".into(),
            title: None,
            working_directory: None,
            requested_options: None,
        })
        .unwrap();
    wait_until(|| runtime.active_turn(&started.invocation_id).is_ok());
    application
        .steer_session(SteerAgentSessionCommand {
            session_id: started.session_id.clone(),
            invocation_id: started.invocation_id,
            input_id: "finish".into(),
            text: "FINISH".into(),
        })
        .unwrap();
    wait_until(|| notifier.successor.lock().unwrap().is_some());
    assert!(notifier.successor.lock().unwrap().as_ref().unwrap().is_ok());
    assert_eq!(
        application
            .load_session(&started.session_id)
            .unwrap()
            .invocations
            .len(),
        2
    );
    runtime.shutdown().unwrap();
}
#[derive(Default)]
struct Factory(Mutex<Vec<Arc<FakeChild>>>);
impl ChildProcessFactory for Factory {
    fn spawn(&self, _: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        let child = Arc::new(FakeChild::default());
        self.0.lock().unwrap().push(child.clone());
        Ok(SpawnedProcess {
            child: child.clone(),
            stdout: Box::new(Reader(child)),
            stderr: Box::new(io::Cursor::new(Vec::<u8>::new())),
        })
    }
}
#[derive(Default)]
struct Notifier(Mutex<Vec<AgentSessionNotification>>);
impl AgentSessionNotifier for Notifier {
    fn notify(&self, event: AgentSessionNotification) -> Result<(), String> {
        self.0.lock().unwrap().push(event);
        Ok(())
    }
}
fn wait_until(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !check() {
        assert!(Instant::now() < deadline, "condition did not become true");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn application_steering_and_approval_keep_one_invocation_and_survive_reopen() {
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("sessions.sqlite");
    let repository = Arc::new(
        SqliteAgentSessionRepository::new(crate::storage::open_active_database(&database).unwrap())
            .unwrap(),
    );
    let factory = Arc::new(Factory::default());
    let runtime = Arc::new(CodexAppServerRuntime::new("fake-codex", factory.clone()));
    let notifier = Arc::new(Notifier::default());
    let providers = Arc::new(SystemAgentSessionProviders);
    let application = AgentSessionApplication::new(
        repository.clone(),
        runtime.clone(),
        notifier.clone(),
        providers.clone(),
        providers,
        None,
    )
    .with_workspaces(
        SessionWorkspaces::new(folder.path().join("product"), "test-database".into()).unwrap(),
    );
    let started = application
        .send_message(SendAgentSessionMessageCommand {
            session_id: None,
            submitted_text: "Wait for steering".into(),
            title: None,
            working_directory: None,
            requested_options: None,
        })
        .unwrap();
    wait_until(|| {
        application
            .session_interactions(&started.session_id)
            .unwrap()
            .iter()
            .any(|i| i.kind == "request")
    });
    let request = application
        .session_interactions(&started.session_id)
        .unwrap()
        .remove(0);
    assert!(application
        .respond_to_runtime_request(RespondToRuntimeRequestCommand {
            session_id: started.session_id.clone(),
            invocation_id: started.invocation_id.clone(),
            request_id: request.id.clone(),
            response: json!({"decision":"acceptForSession"})
        })
        .is_err());
    let response = RespondToRuntimeRequestCommand {
        session_id: started.session_id.clone(),
        invocation_id: started.invocation_id.clone(),
        request_id: request.id.clone(),
        response: json!({"decision":"accept"}),
    };
    application
        .respond_to_runtime_request(response.clone())
        .unwrap();
    assert!(application.respond_to_runtime_request(response).is_err());
    let child = factory.0.lock().unwrap()[0].clone();
    child.output(json!({"id":"question-native","method":"item/tool/requestUserInput","params":{"threadId":"thread-one","turnId":"turn-one","questions":[{"id":"choice","question":"Choose one","isOther":false,"options":[{"label":"One","description":"First option"}]}]}}));
    wait_until(|| {
        application
            .session_interactions(&started.session_id)
            .unwrap()
            .iter()
            .any(|i| i.kind == "request" && i.state == "pending")
    });
    let question = application
        .session_interactions(&started.session_id)
        .unwrap()
        .into_iter()
        .find(|i| i.kind == "request" && i.state == "pending")
        .unwrap();
    application
        .respond_to_runtime_request(RespondToRuntimeRequestCommand {
            session_id: started.session_id.clone(),
            invocation_id: started.invocation_id.clone(),
            request_id: question.id,
            response: json!({"answers":{"choice":{"answers":["One"]}}}),
        })
        .unwrap();
    assert!(child
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "question-native"
            && r["result"]["answers"]["choice"]["answers"][0] == "One"));
    child.output(json!({"id":"unsupported-native","method":"future/request","params":{}}));
    wait_until(|| {
        child
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "unsupported-native" && r["error"]["code"] == -32601)
    });
    for (id, text) in [("input-one", "Continue"), ("input-two", "FINISH")] {
        let command = SteerAgentSessionCommand {
            session_id: started.session_id.clone(),
            invocation_id: started.invocation_id.clone(),
            input_id: id.into(),
            text: text.into(),
        };
        assert_eq!(
            application.steer_session(command.clone()).unwrap().state,
            "accepted"
        );
        assert_eq!(
            application.steer_session(command).unwrap().state,
            "accepted"
        );
    }
    wait_until(|| {
        application
            .load_session(&started.session_id)
            .unwrap()
            .invocations[0]
            .invocation
            .status
            .is_terminal()
    });
    runtime.shutdown().unwrap();
    let history = application.load_session(&started.session_id).unwrap();
    assert_eq!(history.invocations.len(), 1);
    let answer = history.invocations[0]
        .events
        .iter()
        .find(|e| {
            e.normalized
                .as_ref()
                .is_some_and(|n| n.text.as_deref() == Some("STEERED"))
        })
        .unwrap();
    assert_eq!(answer.raw_payload["native"]["method"], "item/completed");
    assert_eq!(
        answer.raw_payload["native"]["params"]["item"]["id"],
        "answer"
    );
    assert_eq!(
        history
            .session
            .runtime_binding
            .external_context_id
            .unwrap()
            .as_str(),
        "thread-one"
    );
    assert!(history.invocations[0]
        .events
        .iter()
        .any(|e| e.raw_payload["kind"] == "runtime_process_exit"));
    assert_eq!(
        notifier
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|n| matches!(n, AgentSessionNotification::SteeringAccepted { .. }))
            .count(),
        2
    );
    let child = factory.0.lock().unwrap()[0].clone();
    assert_eq!(
        child
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r["method"] == "turn/steer")
            .count(),
        2
    );
    let reopened =
        SqliteAgentSessionRepository::new(crate::storage::open_active_database(&database).unwrap())
            .unwrap();
    let durable = project_interactions(
        &reopened
            .load_session_history(&started.session_id)
            .unwrap()
            .unwrap(),
    );
    assert_eq!(
        durable
            .iter()
            .filter(|i| i.kind == "steering" && i.state == "accepted")
            .count(),
        2
    );
    assert_eq!(
        durable
            .iter()
            .filter(|i| i.kind == "request" && i.state == "answered")
            .count(),
        2
    );
    assert!(durable
        .iter()
        .any(|i| i.kind == "request" && i.state == "unsupported"));
}
