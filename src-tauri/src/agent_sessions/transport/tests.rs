use super::{
    dto::AgentSessionUpdateDto, AgentSessionEventPublisher, TauriAgentSessionNotifier,
    AGENT_SESSION_UPDATE_EVENT,
};
use crate::agent_sessions::{
    application::{AgentSessionNotification, AgentSessionNotifier},
    domain::{AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeEventSource, AgentSessionId},
};
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn tauri_notifier_emits_correlated_persisted_update_dto() {
    let publisher = Arc::new(FakePublisher::default());
    let notifier = TauriAgentSessionNotifier {
        publisher: publisher.clone(),
        completion_sound: None,
    };
    notifier
        .notify(AgentSessionNotification::EventPersisted {
            session_id: AgentSessionId::new("session-1").expect("session ID"),
            event: AgentRuntimeEvent {
                id: AgentRuntimeEventId::new("event-1").expect("event ID"),
                invocation_id: crate::agent_sessions::domain::AgentInvocationId::new(
                    "invocation-1",
                )
                .expect("invocation ID"),
                sequence: 0,
                source: AgentRuntimeEventSource::Stdout,
                raw_payload: json!({"type": "test"}),
                normalized: None,
                recorded_at: Utc.with_ymd_and_hms(2026, 7, 10, 12, 0, 0).unwrap(),
            },
        })
        .expect("emit");

    let emitted = publisher.emitted.lock().expect("emitted");
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].0, AGENT_SESSION_UPDATE_EVENT);
    assert!(matches!(
        &emitted[0].1,
        AgentSessionUpdateDto::EventPersisted {
            session_id,
            invocation_id,
            event,
        } if session_id.as_str() == "session-1"
            && invocation_id.as_str() == "invocation-1"
            && event.id.as_str() == "event-1"
    ));
    let serialized = serde_json::to_value(&emitted[0].1).expect("serialize update");
    assert_eq!(serialized["kind"], "event_persisted");
    assert_eq!(serialized["sessionId"], "session-1");
    assert_eq!(serialized["invocationId"], "invocation-1");
}

#[derive(Default)]
struct FakePublisher {
    emitted: Mutex<Vec<(String, AgentSessionUpdateDto)>>,
}

#[test]
fn terminal_updates_carry_the_node_sound_policy_without_suppressing_updates() {
    use crate::agent_sessions::domain::{AgentInvocation, AgentInvocationId, AgentInvocationInputProvenance, AgentInvocationStatus, AgentRuntimeOptions};
    let publisher = Arc::new(FakePublisher::default());
    let notifier = TauriAgentSessionNotifier { publisher:publisher.clone(),completion_sound:Some(Arc::new(|id| id != "silent")) };
    let at = Utc.with_ymd_and_hms(2026,10,12,12,0,0).unwrap();
    for (name,expected) in [("silent",false),("normal",true)] {
        let session = AgentSessionId::new(name).unwrap();
        let invocation = AgentInvocation {id:AgentInvocationId::new(format!("turn-{name}")).unwrap(),session_id:session.clone(),submitted_text:"Test".into(),attachments:vec![],
            input_provenance:AgentInvocationInputProvenance::Application,status:AgentInvocationStatus::Completed,requested_options:AgentRuntimeOptions::default(),effective_options:None,
            started_at:Some(at),completed_at:Some(at),exit_code:Some(0),signal:None,runtime_error:None,diagnostics:vec![],created_at:at,updated_at:at};
        notifier.notify(AgentSessionNotification::InvocationTerminal {session_id:session,invocation}).unwrap();
        let emitted = publisher.emitted.lock().unwrap();
        let payload=serde_json::to_value(&emitted.last().unwrap().1).unwrap();
        assert_eq!(payload["completionSound"],expected);
        assert_eq!(payload["kind"],"invocation_terminal");
        assert_eq!(payload["sessionId"],name);
    }
    assert_eq!(publisher.emitted.lock().unwrap().len(),2);
}

impl AgentSessionEventPublisher for FakePublisher {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String> {
        self.emitted
            .lock()
            .expect("emitted")
            .push((event.to_string(), payload));
        Ok(())
    }
}
