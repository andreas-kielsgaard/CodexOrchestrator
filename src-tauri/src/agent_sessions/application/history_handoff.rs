//! Context for a provider that did not take part in all of a Session's conversation. Native
//! conversation IDs only spare a provider from re-reading history; the Session log is the record.
use crate::agent_sessions::{
    domain::AgentInvocationId,
    ports::{AgentSessionHistory, InitialPromptPrefix},
};

/// What the destination provider has not seen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Handoff {
    /// The provider continues its own conversation, which holds everything.
    None,
    /// The provider continues its own conversation, which ended with this invocation.
    Since(Option<AgentInvocationId>),
    /// The provider starts a conversation: it needs the Session's initial prompt prefix and every
    /// earlier turn.
    Full,
}

pub(crate) const HANDOFF_SOURCE: &str = "orchid_provider_handoff";

/// Combines the handoff with any prefix the caller supplied for this message into the one prefix a
/// provider delivers before the prompt.
pub(crate) fn handoff_prefix(
    history: &AgentSessionHistory,
    current: &AgentInvocationId,
    handoff: &Handoff,
    initial: Option<&InitialPromptPrefix>,
    caller: Option<InitialPromptPrefix>,
) -> Option<InitialPromptPrefix> {
    let (since, initial) = match handoff {
        Handoff::None => return caller,
        Handoff::Since(since) => (since.as_ref(), None),
        Handoff::Full => (None, initial),
    };
    let earlier = history
        .invocations
        .iter()
        .take_while(|entry| &entry.invocation.id != current);
    let turns: Vec<_> = match since {
        Some(since) => earlier
            .skip_while(|entry| &entry.invocation.id != since)
            .skip(1)
            .filter(|entry| entry.invocation.status.is_terminal())
            .collect(),
        None => earlier
            .filter(|entry| entry.invocation.status.is_terminal())
            .collect(),
    };
    if turns.is_empty() && initial.is_none() {
        return caller;
    }
    let mut content = String::new();
    if let Some(initial) = initial {
        content.push_str(&format!(
            "<first_message_context source=\"{}\" version=\"{}\">\n{}\n</first_message_context>\n\n",
            initial.source, initial.version, initial.content
        ));
    }
    if !turns.is_empty() {
        content.push_str(
            "This Orchid Session continued with another agent. These are the turns you have not seen, oldest first.\n<conversation_history>\n",
        );
        for entry in turns {
            content.push_str(&format!(
                "<user_message>\n{}\n</user_message>\n",
                entry.invocation.submitted_text
            ));
            if let Some(reply) = entry.final_reply() {
                content.push_str(&format!("<agent_reply>\n{reply}\n</agent_reply>\n"));
            }
        }
        content.push_str("</conversation_history>\n");
    }
    if let Some(caller) = caller {
        content.push_str(&format!(
            "\n<message_context source=\"{}\" version=\"{}\">\n{}\n</message_context>\n",
            caller.source, caller.version, caller.content
        ));
    }
    Some(InitialPromptPrefix {
        source: HANDOFF_SOURCE.into(),
        version: 1,
        content: content.trim_end().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::{
        domain::*,
        ports::{AgentInvocationHistory, AgentSessionHistory},
    };
    use chrono::Utc;
    use serde_json::json;

    fn invocation(id: &str, text: &str, status: AgentInvocationStatus) -> AgentInvocation {
        AgentInvocation {
            id: AgentInvocationId::new(id).unwrap(),
            session_id: AgentSessionId::new("session").unwrap(),
            submitted_text: text.into(),
            input_provenance: AgentInvocationInputProvenance::User,
            status,
            requested_options: Default::default(),
            effective_options: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            signal: None,
            runtime_error: None,
            diagnostics: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn reply(invocation_id: &str, text: &str) -> AgentRuntimeEvent {
        AgentRuntimeEvent {
            id: AgentRuntimeEventId::new(format!("{invocation_id}-reply")).unwrap(),
            invocation_id: AgentInvocationId::new(invocation_id).unwrap(),
            sequence: 1,
            source: AgentRuntimeEventSource::Stdout,
            raw_payload: json!({}),
            normalized: Some(NormalizedRuntimeEvent {
                kind: NormalizedRuntimeEventKind::AgentMessage,
                text: Some(text.into()),
                external_context_id: None,
                usage: None,
                details: Some(json!({"role": agent_message_role::FINAL})),
                tool_activity: None,
            }),
            recorded_at: Utc::now(),
        }
    }

    fn history() -> AgentSessionHistory {
        let entry = |id: &str, text: &str, status, answer: Option<&str>| AgentInvocationHistory {
            invocation: invocation(id, text, status),
            launch_accepted_at: None,
            events: answer.map(|answer| vec![reply(id, answer)]).unwrap_or_default(),
            import_provenance: None,
        };
        AgentSessionHistory {
            session: AgentSession {
                execution_target: None,
                workspace_origin: None,
                id: AgentSessionId::new("session").unwrap(),
                title: "Session".into(),
                availability: AgentSessionAvailability::Available,
                runtime_binding: AgentRuntimeBinding {
                    external_context_id: None,
                    runtime_version: None,
                },
                working_directory: None,
                requested_options: Default::default(),
                session_profile: None,
                harness_version: None,
                assigned_identity: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            invocations: vec![
                entry("one", "First question", AgentInvocationStatus::Completed, Some("First answer")),
                entry("two", "Second question", AgentInvocationStatus::Completed, Some("Second answer")),
                entry("three", "Current question", AgentInvocationStatus::Pending, None),
            ],
        }
    }

    fn id(value: &str) -> AgentInvocationId {
        AgentInvocationId::new(value).unwrap()
    }

    fn prefix(source: &str, content: &str) -> InitialPromptPrefix {
        InitialPromptPrefix {
            source: source.into(),
            version: 2,
            content: content.into(),
        }
    }

    #[test]
    fn a_new_provider_receives_the_initial_prefix_and_every_earlier_turn() {
        let initial = prefix("harness", "Role instructions");
        let handoff = handoff_prefix(&history(), &id("three"), &Handoff::Full, Some(&initial), None)
            .unwrap();
        assert_eq!(handoff.source, HANDOFF_SOURCE);
        let content = handoff.content;
        assert!(content.starts_with("<first_message_context source=\"harness\" version=\"2\">\nRole instructions"));
        for text in ["First question", "First answer", "Second question", "Second answer"] {
            assert!(content.contains(text), "{text}");
        }
        assert!(!content.contains("Current question"));
    }

    #[test]
    fn a_returning_provider_receives_only_the_turns_it_missed() {
        let initial = prefix("harness", "Role instructions");
        let content = handoff_prefix(
            &history(),
            &id("three"),
            &Handoff::Since(Some(id("one"))),
            Some(&initial),
            None,
        )
        .unwrap()
        .content;
        assert!(!content.contains("Role instructions"));
        assert!(!content.contains("First question"));
        assert!(content.contains("Second question") && content.contains("Second answer"));
    }

    #[test]
    fn nothing_to_hand_over_keeps_the_callers_prefix() {
        let caller = prefix("caller", "Message context");
        assert_eq!(
            handoff_prefix(&history(), &id("three"), &Handoff::Since(Some(id("two"))), None, Some(caller.clone())),
            Some(caller.clone())
        );
        assert_eq!(
            handoff_prefix(&history(), &id("three"), &Handoff::None, None, Some(caller.clone())),
            Some(caller.clone())
        );
        let combined = handoff_prefix(&history(), &id("three"), &Handoff::Full, None, Some(caller))
            .unwrap()
            .content;
        assert!(combined.contains("<message_context source=\"caller\" version=\"2\">\nMessage context"));
    }
}
