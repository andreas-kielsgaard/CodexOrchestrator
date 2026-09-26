//! The Claude runtime against recorded streams. A fake process plays a recording in segments:
//! each prompt, answer or interrupt Orchid writes releases the next segment.
use super::ClaudeRuntime;
use crate::{
    contracts::{control::*, domain::*, ports::*, RuntimeInteractionResponse, RuntimeRequestKind},
    processes::*,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, Read},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/providers/claude/fixtures/claude-code-2.1.282"
);

/// Claude's recorded output for one scenario.
pub(super) fn fixture(name: &str) -> Vec<Value> {
    std::fs::read_to_string(format!("{FIXTURES}/{name}.jsonl"))
        .expect("fixture exists")
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture line is JSON"))
        .collect()
}

/// Splits a recording where Claude waits on Orchid: after each permission prompt, and after the
/// first message for which `pause` holds.
fn segments(messages: Vec<Value>, pause: impl Fn(&Value) -> bool) -> VecDeque<Vec<Value>> {
    let mut segments = VecDeque::from([Vec::new()]);
    let mut paused = false;
    for message in messages {
        let boundary = message["type"] == "control_request" || (!paused && pause(&message));
        paused |= pause(&message);
        segments.back_mut().unwrap().push(message);
        if boundary {
            segments.push_back(Vec::new());
        }
    }
    segments
}

#[derive(Default)]
struct Wire {
    bytes: VecDeque<u8>,
    exited: bool,
}

#[derive(Default)]
struct FakeClaude {
    wire: Mutex<Wire>,
    changed: Condvar,
    segments: Mutex<VecDeque<Vec<Value>>>,
    input: Mutex<Vec<Value>>,
}

impl FakeClaude {
    fn play_next(&self) {
        let segment = self
            .segments
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default();
        let mut wire = self.wire.lock().unwrap();
        for message in segment {
            wire.bytes.extend(message.to_string().bytes());
            wire.bytes.push_back(b'\n');
        }
        self.changed.notify_all();
    }
}

struct Reader(Arc<FakeClaude>);
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

impl SupervisedChild for FakeClaude {
    fn write_input(&self, bytes: &[u8]) -> io::Result<()> {
        let value: Value = serde_json::from_slice(bytes)?;
        self.input.lock().unwrap().push(value.clone());
        let releases = value["type"] == "user"
            || value["type"] == "control_response"
            || value["request"]["subtype"] == "interrupt";
        if releases {
            self.play_next();
        }
        Ok(())
    }
    fn close_input(&self) -> io::Result<()> {
        self.terminate()
    }
    fn try_wait(&self) -> io::Result<Option<ProcessExit>> {
        Ok(self.wire.lock().unwrap().exited.then(ProcessExit::default))
    }
    fn terminate(&self) -> io::Result<()> {
        self.wire.lock().unwrap().exited = true;
        self.changed.notify_all();
        Ok(())
    }
    fn wait_after_termination(&self) -> io::Result<ProcessExit> {
        Ok(ProcessExit::default())
    }
}

struct Factory {
    child: Arc<FakeClaude>,
    specs: Mutex<Vec<ProcessLaunchSpec>>,
}

impl ChildProcessFactory for Factory {
    fn spawn(&self, spec: &ProcessLaunchSpec) -> io::Result<SpawnedProcess> {
        self.specs.lock().unwrap().push(spec.clone());
        Ok(SpawnedProcess {
            child: self.child.clone(),
            stdout: Box::new(Reader(self.child.clone())),
            stderr: Box::new(io::Cursor::new(Vec::<u8>::new())),
        })
    }
}

struct Sink {
    updates: Mutex<Vec<RuntimeUpdate>>,
    patience: Duration,
}

impl Default for Sink {
    fn default() -> Self {
        Self::waiting(Duration::from_secs(5))
    }
}
impl AgentRuntimeUpdateSink for Sink {
    fn emit_update(
        &self,
        _: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        self.updates.lock().unwrap().push(update);
        Ok(())
    }
    fn report_delivery_failure(&self, _: &AgentInvocationId, _: RuntimeUpdateDeliveryFailure) {}
}

impl Sink {
    fn waiting(patience: Duration) -> Self {
        Self {
            updates: Mutex::default(),
            patience,
        }
    }

    fn wait_for(&self, found: impl Fn(&[RuntimeUpdate]) -> bool) -> Vec<RuntimeUpdate> {
        let deadline = Instant::now() + self.patience;
        loop {
            let updates = self.updates.lock().unwrap().clone();
            if found(&updates) {
                return updates;
            }
            assert!(
                Instant::now() < deadline,
                "timed out; updates: {updates:#?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn outcome(&self) -> RuntimeInvocationOutcome {
        let updates = self.wait_for(|updates| {
            updates
                .iter()
                .any(|u| matches!(u, RuntimeUpdate::Finished(_)))
        });
        updates
            .into_iter()
            .find_map(|update| match update {
                RuntimeUpdate::Finished(outcome) => Some(outcome),
                _ => None,
            })
            .unwrap()
    }

    fn normalized(&self) -> Vec<NormalizedRuntimeEvent> {
        self.updates
            .lock()
            .unwrap()
            .iter()
            .filter_map(|update| match update {
                RuntimeUpdate::Event(event) => event.normalized.clone(),
                _ => None,
            })
            .collect()
    }

    fn controls(&self) -> Vec<RuntimeControlRecord> {
        self.updates
            .lock()
            .unwrap()
            .iter()
            .filter_map(|update| match update {
                RuntimeUpdate::Event(event) => {
                    RuntimeControlRecord::from_event(event.source, &event.raw_payload)
                }
                _ => None,
            })
            .collect()
    }
}

struct Harness {
    runtime: ClaudeRuntime,
    child: Arc<FakeClaude>,
    factory: Arc<Factory>,
    sink: Arc<Sink>,
}

impl Harness {
    fn new(segments: VecDeque<Vec<Value>>) -> Self {
        let child = Arc::new(FakeClaude {
            segments: Mutex::new(segments),
            ..Default::default()
        });
        let factory = Arc::new(Factory {
            child: child.clone(),
            specs: Mutex::default(),
        });
        Self {
            runtime: ClaudeRuntime::new("claude", factory.clone()),
            child,
            factory,
            sink: Arc::new(Sink::default()),
        }
    }

    fn scenario(name: &str) -> Self {
        Self::new(segments(fixture(name), |_| false))
    }

    fn start(&self, extension: Option<RuntimeLaunchExtension>) {
        self.runtime
            .start_invocation(request(extension), self.sink.clone())
            .unwrap();
    }

    fn args(&self) -> Vec<String> {
        self.factory.specs.lock().unwrap()[0].args.clone()
    }

    fn written(&self) -> Vec<Value> {
        self.child.input.lock().unwrap().clone()
    }

    fn opened_request(&self) -> crate::contracts::RuntimeRequest {
        self.sink.wait_for(|updates| updates.iter().any(|update| matches!(update, RuntimeUpdate::Event(e) if e.raw_payload["kind"] == "runtime_request_opened")));
        self.sink
            .controls()
            .into_iter()
            .find_map(|record| match record {
                RuntimeControlRecord::RequestOpened { request } => Some(request),
                _ => None,
            })
            .unwrap()
    }
}

fn invocation_id() -> AgentInvocationId {
    AgentInvocationId::new("invocation").unwrap()
}

fn request(extension: Option<RuntimeLaunchExtension>) -> RuntimeInvocationRequest {
    RuntimeInvocationRequest {
        session_id: AgentSessionId::new("session").unwrap(),
        invocation_id: invocation_id(),
        submitted_text: "Reply with exactly the word: ok".into(),
        working_directory: Some("/work".into()),
        options: AgentRuntimeOptions {
            model: Some("haiku".into()),
            sandbox: None,
        },
        launch_extension: extension,
    }
}

fn messages(events: &[NormalizedRuntimeEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|event| event.kind == NormalizedRuntimeEventKind::AgentMessage)
        .map(|event| {
            (
                event.agent_message_role().unwrap().into(),
                event.text.clone().unwrap(),
            )
        })
        .collect()
}

#[test]
fn a_reply_establishes_the_conversation_and_completes_with_usage() {
    let harness = Harness::scenario("reply");
    harness.start(None);
    let outcome = harness.sink.outcome();
    assert_eq!(outcome.status, AgentInvocationTerminalStatus::Completed);

    let args = harness.args();
    let session = &args[args.iter().position(|arg| arg == "--session-id").unwrap() + 1];
    let events = harness.sink.normalized();
    assert_eq!(
        events[0].kind,
        NormalizedRuntimeEventKind::RuntimeContextEstablished
    );
    assert_eq!(
        events[0].external_context_id.as_ref().unwrap().as_str(),
        session
    );
    assert_eq!(messages(&events), [("final".to_string(), "ok".to_string())]);
    let usage = events.iter().find_map(|event| event.usage.clone()).unwrap();
    assert!(usage.input_tokens > usage.cached_input_tokens && usage.output_tokens.is_some());
    assert_eq!(
        events.last().unwrap().kind,
        NormalizedRuntimeEventKind::InvocationCompleted
    );

    let written = harness.written();
    assert_eq!(written[0]["request"]["subtype"], "initialize");
    assert_eq!(
        written[1]["message"]["content"],
        "Reply with exactly the word: ok"
    );
    assert_eq!(written[1]["session_id"].as_str(), Some(session.as_str()));
    let target = harness
        .sink
        .controls()
        .into_iter()
        .find_map(|record| match record {
            RuntimeControlRecord::TurnActive { target } => Some(target),
            _ => None,
        });
    assert_eq!(target.unwrap().thread_id, *session);
}

#[test]
fn resuming_continues_the_named_conversation() {
    let harness = Harness::scenario("reply");
    harness
        .runtime
        .resume_invocation(
            request(None),
            ExternalRuntimeContextId::new("earlier").unwrap(),
            harness.sink.clone(),
        )
        .unwrap();
    harness.sink.outcome();
    let args = harness.args().join(" ");
    assert!(args.contains("--resume earlier") && !args.contains("--session-id"));
    assert_eq!(harness.written()[1]["session_id"], "earlier");
}

#[test]
fn preparation_waits_for_delivery_before_writing_the_prompt() {
    let harness = Harness::scenario("reply");
    let ready = harness
        .runtime
        .prepare_invocation(request(None), None, harness.sink.clone())
        .unwrap();
    assert_eq!(ready.working_directory, "/work");
    assert_eq!(harness.written().len(), 1);
    harness
        .runtime
        .deliver_prepared_invocation(&invocation_id())
        .unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert!(harness
        .runtime
        .deliver_prepared_invocation(&invocation_id())
        .is_err());
}

#[test]
fn tool_calls_become_tool_activity() {
    let harness = Harness::scenario("tool");
    harness.start(None);
    harness.sink.outcome();
    let tools: Vec<_> = harness
        .sink
        .normalized()
        .into_iter()
        .filter_map(|event| Some((event.tool_activity?, event.text)))
        .collect();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].0.kind, ToolActivityKind::Command);
    assert_eq!(tools[0].0.phase, ToolActivityPhase::Started);
    assert_eq!(tools[0].1.as_deref(), Some("echo orchid-probe"));
    assert_eq!(tools[1].0.phase, ToolActivityPhase::Completed);
    assert_eq!(
        tools[1].0.result_classification,
        ToolResultClassification::Succeeded
    );
    assert_eq!(tools[0].0.item_id, tools[1].0.item_id);
}

#[test]
fn an_approved_permission_prompt_answers_claude_and_continues() {
    let harness = Harness::scenario("approval");
    harness.start(None);
    let request = harness.opened_request();
    assert_eq!(request.kind, RuntimeRequestKind::Approval);
    harness
        .runtime
        .respond(
            &invocation_id(),
            &request.id,
            RuntimeInteractionResponse::Choose {
                choice_id: "allow".into(),
            },
        )
        .unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    let answer = harness
        .written()
        .into_iter()
        .find(|message| message["type"] == "control_response")
        .unwrap();
    let prompt = fixture("approval")
        .into_iter()
        .find(|message| message["type"] == "control_request")
        .unwrap();
    assert_eq!(answer["response"]["request_id"], prompt["request_id"]);
    assert_eq!(answer["response"]["response"]["behavior"], "allow");
    assert!(harness
        .runtime
        .respond(
            &invocation_id(),
            &request.id,
            RuntimeInteractionResponse::Choose {
                choice_id: "allow".into()
            }
        )
        .is_err());
}

#[test]
fn a_question_is_answered_through_the_permission_prompt() {
    let harness = Harness::scenario("question");
    harness.start(None);
    let request = harness.opened_request();
    assert_eq!(request.kind, RuntimeRequestKind::Questions);
    let answers = BTreeMap::from([("q1".to_string(), vec!["Blue".to_string()])]);
    harness
        .runtime
        .respond(
            &invocation_id(),
            &request.id,
            RuntimeInteractionResponse::Answer { answers },
        )
        .unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert_eq!(
        messages(&harness.sink.normalized()),
        [("final".to_string(), "Blue".to_string())]
    );
}

#[test]
fn cancel_interrupts_the_turn_and_finishes_canceled() {
    let harness = Harness::new(segments(fixture("interrupt"), |message| {
        message["type"] == "assistant"
    }));
    harness.start(None);
    harness.sink.wait_for(|updates| updates.iter().any(|update| matches!(update, RuntimeUpdate::Event(e) if e.raw_payload["type"] == "assistant")));
    harness.runtime.cancel_invocation(&invocation_id()).unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Canceled
    );
    assert!(harness
        .written()
        .iter()
        .any(|message| message["request"]["subtype"] == "interrupt"));
}

#[test]
fn steering_joins_the_running_turn_and_completes_once() {
    let harness = Harness::new(segments(fixture("steer"), |message| {
        message["message"]["content"][0]["type"] == "tool_use"
    }));
    harness.start(None);
    let target = harness.runtime.active_turn(&invocation_id()).unwrap();
    harness.sink.wait_for(|updates| {
        updates.iter().any(|update| matches!(update, RuntimeUpdate::Event(e) if e.normalized.as_ref().is_some_and(|n| n.tool_activity.is_some())))
    });
    harness
        .runtime
        .steer(
            &invocation_id(),
            &target,
            "input",
            "Also: end your final reply with the word banana.",
        )
        .unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert_eq!(
        messages(&harness.sink.normalized()),
        [("final".to_string(), "done banana".to_string())]
    );
    let finished = harness
        .sink
        .updates
        .lock()
        .unwrap()
        .iter()
        .filter(|u| matches!(u, RuntimeUpdate::Finished(_)))
        .count();
    assert_eq!(finished, 1);
}

#[test]
fn a_result_before_a_steering_message_is_taken_in_does_not_finish() {
    let mut reply = fixture("reply");
    let result = reply.pop().unwrap();
    let steered = json!({"type":"user","message":{"role":"user","content":"more"},"parent_tool_use_id":null,"isReplay":true});
    let answer = json!({"type":"assistant","parent_tool_use_id":null,"message":{"content":[{"type":"text","text":"more done"}]}});
    let harness = Harness::new(VecDeque::from([
        reply,
        vec![result.clone(), steered, answer, result],
    ]));
    harness.start(None);
    let target = harness.runtime.active_turn(&invocation_id()).unwrap();
    harness.sink.wait_for(|updates| updates.iter().any(|update| matches!(update, RuntimeUpdate::Event(e) if e.raw_payload["type"] == "assistant")));
    harness
        .runtime
        .steer(&invocation_id(), &target, "input", "more")
        .unwrap();
    assert_eq!(
        harness.sink.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert_eq!(
        messages(&harness.sink.normalized()),
        [
            ("intermediate".to_string(), "ok".to_string()),
            ("final".to_string(), "more done".to_string())
        ]
    );
}

#[test]
fn an_unavailable_required_server_fails_the_invocation() {
    let harness = Harness::scenario("reply");
    harness.start(Some(RuntimeLaunchExtension {
        managed_mcp_servers: vec![RuntimeManagedMcpServer {
            name: "orchid".into(),
            url: "http://127.0.0.1/mcp".into(),
            bearer_token: None,
            enabled_tools: None,
            required: true,
        }],
        ..Default::default()
    }));
    let outcome = harness.sink.outcome();
    assert_eq!(outcome.status, AgentInvocationTerminalStatus::Failed);
    assert!(outcome.runtime_error.unwrap().message.contains("orchid"));
}

#[test]
fn a_failed_result_fails_the_invocation_with_its_errors() {
    let mut messages = fixture("interrupt");
    messages.retain(|message| message["type"] != "user" || message["isReplay"] == true);
    let harness = Harness::new(segments(messages, |_| false));
    harness.start(None);
    let outcome = harness.sink.outcome();
    assert_eq!(outcome.status, AgentInvocationTerminalStatus::Failed);
    assert!(outcome
        .runtime_error
        .unwrap()
        .message
        .contains("ede_diagnostic"));
}

/// Drives the installed, signed-in `claude` CLI. Paid: runs only when `ORCHID_CLAUDE_LIVE=true`,
/// with `cargo test -p orchid-engine live_claude_code -- --ignored`.
#[test]
#[ignore = "runs the installed Claude Code CLI against the signed-in account"]
fn live_claude_code() {
    if std::env::var("ORCHID_CLAUDE_LIVE").as_deref() != Ok("true") {
        eprintln!("skipped: set ORCHID_CLAUDE_LIVE=true to run");
        return;
    }
    let models = super::discovery::models("claude", &[]).expect("initialize reports models");
    assert!(!models.is_empty());
    let work = tempfile::tempdir().unwrap();
    let runtime = ClaudeRuntime::system("claude");
    let live_request = |id: &str, text: &str| RuntimeInvocationRequest {
        invocation_id: AgentInvocationId::new(id).unwrap(),
        submitted_text: text.into(),
        working_directory: Some(work.path().to_string_lossy().into_owned()),
        ..request(None)
    };
    let invocation = |id: &str| AgentInvocationId::new(id).unwrap();
    let reply = |sink: &Sink| {
        messages(&sink.normalized())
            .last()
            .map(|(_, text)| text.clone())
    };

    let first = Arc::new(Sink::waiting(Duration::from_secs(180)));
    runtime
        .start_invocation(
            live_request("first", "Reply with exactly the word: ok"),
            first.clone(),
        )
        .unwrap();
    assert_eq!(
        first.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert!(reply(&first).unwrap().to_lowercase().contains("ok"));
    let conversation = first.normalized()[0].external_context_id.clone().unwrap();

    let resumed = Arc::new(Sink::waiting(Duration::from_secs(180)));
    runtime
        .resume_invocation(
            live_request(
                "resumed",
                "Which single word did you reply with last time? Answer with that word only.",
            ),
            conversation,
            resumed.clone(),
        )
        .unwrap();
    assert_eq!(
        resumed.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert!(reply(&resumed).unwrap().to_lowercase().contains("ok"));

    let asked = Arc::new(Sink::waiting(Duration::from_secs(180)));
    runtime
        .start_invocation(
            live_request("asked", "Use the AskUserQuestion tool once to ask which color I prefer, offering exactly red and blue. Then reply with only the chosen color."),
            asked.clone(),
        )
        .unwrap();
    asked.wait_for(|updates| updates.iter().any(|u| matches!(u, RuntimeUpdate::Event(e) if e.raw_payload["kind"] == "runtime_request_opened")));
    let question = asked
        .controls()
        .into_iter()
        .find_map(|record| match record {
            RuntimeControlRecord::RequestOpened { request } => request.questions,
            _ => None,
        })
        .unwrap();
    let blue = question[0]
        .options
        .as_ref()
        .unwrap()
        .iter()
        .find(|o| o.label.to_lowercase() == "blue")
        .unwrap()
        .label
        .clone();
    let request_id = asked
        .controls()
        .into_iter()
        .find_map(|record| match record {
            RuntimeControlRecord::RequestOpened { request } => Some(request.id),
            _ => None,
        });
    runtime
        .respond(
            &invocation("asked"),
            &request_id.unwrap(),
            RuntimeInteractionResponse::Answer {
                answers: BTreeMap::from([(question[0].id.clone(), vec![blue])]),
            },
        )
        .unwrap();
    assert_eq!(
        asked.outcome().status,
        AgentInvocationTerminalStatus::Completed
    );
    assert!(reply(&asked).unwrap().to_lowercase().contains("blue"));

    let canceled = Arc::new(Sink::waiting(Duration::from_secs(180)));
    runtime
        .start_invocation(
            live_request(
                "canceled",
                "Count slowly from 1 to 500, one number per line.",
            ),
            canceled.clone(),
        )
        .unwrap();
    canceled.wait_for(|updates| {
        updates
            .iter()
            .any(|u| matches!(u, RuntimeUpdate::Event(e) if e.raw_payload["type"] == "assistant"))
    });
    runtime.cancel_invocation(&invocation("canceled")).unwrap();
    assert_eq!(
        canceled.outcome().status,
        AgentInvocationTerminalStatus::Canceled
    );
}
