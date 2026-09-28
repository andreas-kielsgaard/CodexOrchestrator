//! Context for a provider that did not take part in all of a Session's conversation. Native
//! conversation IDs only spare a provider from re-reading history; the Session log is the record.
use crate::agent_sessions::{
    domain::AgentInvocationId,
    ports::{
        AgentSessionHistory, InitialPromptPrefix, InvocationContextPart, InvocationParticipantRole,
    },
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

/// Builds ordered semantic context without flattening provenance into the current user query.
pub(crate) fn handoff_context(
    history: &AgentSessionHistory,
    current: &AgentInvocationId,
    handoff: &Handoff,
    initial: Option<&InitialPromptPrefix>,
    caller: Option<InitialPromptPrefix>,
) -> Vec<InvocationContextPart> {
    let (since, initial) = match handoff {
        Handoff::None => {
            return caller.into_iter().map(instruction_part).collect();
        }
        Handoff::Since(since) => (since.as_ref(), None),
        Handoff::Full => (None, initial),
    };
    let earlier: Vec<_> = history
        .invocations
        .iter()
        .take_while(|entry| &entry.invocation.id != current)
        .filter(|entry| entry.invocation.status.is_terminal())
        .collect();
    let turns: &[&crate::agent_sessions::ports::AgentInvocationHistory] = match since {
        Some(since) => earlier
            .iter()
            .position(|entry| &entry.invocation.id == since)
            .map(|position| &earlier[position + 1..])
            // A stale or imported cursor must never suppress canonical context.
            .unwrap_or(&earlier),
        None => &earlier,
    };
    let mut context = Vec::new();
    if let Some(initial) = initial {
        context.push(instruction_part(initial.clone()));
    }
    for entry in turns.iter().copied() {
        context.push(InvocationContextPart::MissedConversationTurn {
            invocation_id: entry.invocation.id.clone(),
            role: InvocationParticipantRole::User,
            content: entry.invocation.submitted_text.clone(),
        });
        if let Some(reply) = entry.final_reply() {
            context.push(InvocationContextPart::MissedConversationTurn {
                invocation_id: entry.invocation.id.clone(),
                role: InvocationParticipantRole::Agent,
                content: reply.into(),
            });
        }
    }
    if let Some(caller) = caller {
        context.push(instruction_part(caller));
    }
    context
}

fn instruction_part(prefix: InitialPromptPrefix) -> InvocationContextPart {
    InvocationContextPart::InitialInstructions {
        source: prefix.source,
        version: prefix.version,
        content: prefix.content,
    }
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
            events: answer
                .map(|answer| vec![reply(id, answer)])
                .unwrap_or_default(),
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
                entry(
                    "one",
                    "First question",
                    AgentInvocationStatus::Completed,
                    Some("First answer"),
                ),
                entry(
                    "two",
                    "Second question",
                    AgentInvocationStatus::Completed,
                    Some("Second answer"),
                ),
                entry(
                    "three",
                    "Current question",
                    AgentInvocationStatus::Pending,
                    None,
                ),
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
        let content = serde_json::to_string(&handoff_context(
            &history(),
            &id("three"),
            &Handoff::Full,
            Some(&initial),
            None,
        ))
        .unwrap();
        assert!(content.contains("Role instructions"));
        for text in [
            "First question",
            "First answer",
            "Second question",
            "Second answer",
        ] {
            assert!(content.contains(text), "{text}");
        }
        assert!(!content.contains("Current question"));
    }

    #[test]
    fn a_returning_provider_receives_only_the_turns_it_missed() {
        let initial = prefix("harness", "Role instructions");
        let content = serde_json::to_string(&handoff_context(
            &history(),
            &id("three"),
            &Handoff::Since(Some(id("one"))),
            Some(&initial),
            None,
        ))
        .unwrap();
        assert!(!content.contains("Role instructions"));
        assert!(!content.contains("First question"));
        assert!(content.contains("Second question") && content.contains("Second answer"));
    }

    #[test]
    fn an_unknown_provider_cursor_falls_back_to_all_canonical_turns() {
        let content = serde_json::to_string(&handoff_context(
            &history(),
            &id("three"),
            &Handoff::Since(Some(id("not-in-history"))),
            None,
            None,
        ))
        .unwrap();
        assert!(content.contains("First question") && content.contains("First answer"));
        assert!(content.contains("Second question") && content.contains("Second answer"));
    }

    #[test]
    fn caller_context_is_kept_for_resume_and_full_handoff() {
        let caller = prefix("caller", "Message context");
        let expected = vec![instruction_part(caller.clone())];
        assert_eq!(
            handoff_context(
                &history(),
                &id("three"),
                &Handoff::Since(Some(id("two"))),
                None,
                Some(caller.clone())
            ),
            expected
        );
        assert_eq!(
            handoff_context(
                &history(),
                &id("three"),
                &Handoff::None,
                None,
                Some(caller.clone())
            ),
            expected
        );
        let combined =
            handoff_context(&history(), &id("three"), &Handoff::Full, None, Some(caller));
        assert_eq!(combined.last(), expected.last());
        assert!(combined.len() > expected.len());
    }
}
