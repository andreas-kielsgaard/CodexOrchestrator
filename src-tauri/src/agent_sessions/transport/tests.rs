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

impl AgentSessionEventPublisher for FakePublisher {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String> {
        self.emitted
            .lock()
            .expect("emitted")
            .push((event.to_string(), payload));
        Ok(())
    }
}
