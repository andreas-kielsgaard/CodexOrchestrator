//! Voice owns topology and runtime binding; shared Workflow code owns only presentation contracts.
use super::domain::{ActivityDelivery, Conversation, SpokenInput, VoiceState};
use crate::workflows::extensions::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct VoiceWorkflowSettings {
    pub(crate) direct_reactions: bool,
    pub(crate) notification_sounds: std::collections::BTreeMap<String, bool>,
}
impl Default for VoiceWorkflowSettings {
    fn default() -> Self {
        Self {
            direct_reactions: true,
            notification_sounds: Default::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FlowEvent {
    WorkReady,
    ReactionReady,
    SummaryResult,
    RouterResult,
    Progress,
    OutputRequested,
    ReactionRerouted,
}

struct Edge {
    id: &'static str,
    name: &'static str,
    source: &'static str,
    destination: &'static str,
    event: &'static str,
    condition: &'static str,
    payload: &'static str,
    route: Option<FlowEvent>,
}
const EDGES: &[Edge] = &[
    Edge {
        id: "transcript-jev",
        name: "Current sentences",
        source: "transcript",
        destination: "filter",
        event: "transcript_window",
        condition: "Sentence threshold or voice pause; text remains revisable",
        payload: "Timed sentence revisions",
        route: None,
    },
    Edge {
        id: "jev-buffer",
        name: "Accepted input",
        source: "filter",
        destination: "buffer",
        event: "input_evaluated",
        condition: "Relevance is sticky; JEV separately judges completion and continuation",
        payload: "Typed JEV choices and sentence identities",
        route: None,
    },
    Edge {
        id: "buffer-summary",
        name: "Quick receipt",
        source: "buffer",
        destination: "summary",
        event: "work_ready",
        condition: "Completed work or mixed input",
        payload: "Accepted request and continuation reference",
        route: Some(FlowEvent::WorkReady),
    },
    Edge {
        id: "buffer-router",
        name: "Work routing",
        source: "buffer",
        destination: "router",
        event: "work_ready",
        condition: "Completed work, mixed input, or reaction shortcut disabled",
        payload: "Accepted request and continuation reference",
        route: Some(FlowEvent::WorkReady),
    },
    Edge {
        id: "buffer-digest",
        name: "User context",
        source: "buffer",
        destination: "digest",
        event: "work_ready",
        condition: "Same completed request, delivered in parallel",
        payload: "Accepted request and source/time",
        route: Some(FlowEvent::WorkReady),
    },
    Edge {
        id: "buffer-output",
        name: "Direct reaction",
        source: "buffer",
        destination: "output",
        event: "reaction_ready",
        condition: "Every accepted sentence is a reaction request; shortcut enabled",
        payload: "Accepted request and available discussion context",
        route: Some(FlowEvent::ReactionReady),
    },
    Edge {
        id: "summary-router",
        name: "Summary",
        source: "summary",
        destination: "router",
        event: "summary_result",
        condition: "Receipt/summarization turn ends",
        payload: "Session result and invocation reference",
        route: Some(FlowEvent::SummaryResult),
    },
    Edge {
        id: "summary-digest",
        name: "Summary context",
        source: "summary",
        destination: "digest",
        event: "summary_result",
        condition: "Receipt/summarization turn ends",
        payload: "Session result and invocation reference",
        route: Some(FlowEvent::SummaryResult),
    },
    Edge {
        id: "router-workers",
        name: "Delegate",
        source: "router",
        destination: "workers",
        event: "delegation",
        condition: "Router chooses an existing or new worker through its tools",
        payload: "Task identity, revision and assignment",
        route: None,
    },
    Edge {
        id: "workers-router",
        name: "Return",
        source: "workers",
        destination: "router",
        event: "worker_result",
        condition: "Return to the requesting agent; may also be another worker",
        payload: "Result, assignment and invocation references",
        route: None,
    },
    Edge {
        id: "workers-digest",
        name: "Observations",
        source: "workers",
        destination: "digest",
        event: "worker_observation",
        condition: "Worker feedback, reported outcomes or terminal result",
        payload: "Annotated worker event and task revision",
        route: None,
    },
    Edge {
        id: "router-digest",
        name: "Routing result",
        source: "router",
        destination: "digest",
        event: "router_result",
        condition: "Routing turn ends",
        payload: "Session result and invocation reference",
        route: Some(FlowEvent::RouterResult),
    },
    Edge {
        id: "progress-digest",
        name: "Progress",
        source: "router",
        destination: "digest",
        event: "progress",
        condition: "A conversation member reports relevant progress",
        payload: "Progress text and source/time",
        route: Some(FlowEvent::Progress),
    },
    Edge {
        id: "digest-output",
        name: "Request output",
        source: "digest",
        destination: "output",
        event: "output_requested",
        condition: "Digest decides now is a useful time to speak",
        payload: "Reason, activity references and optional topic",
        route: Some(FlowEvent::OutputRequested),
    },
    Edge {
        id: "output-router",
        name: "Reclassify",
        source: "output",
        destination: "router",
        event: "reaction_rerouted",
        condition: "Output determines the reaction needs actual work",
        payload: "Original request identity and reason",
        route: Some(FlowEvent::ReactionRerouted),
    },
];

fn sessions(c: &Conversation, node: &str) -> Vec<String> {
    if node == "router" {
        return vec![c.main_session_id.clone()];
    }
    if node == "workers" {
        return c.delegations.iter().map(|d| d.session_id.clone()).collect();
    }
    c.pipeline
        .as_ref()
        .and_then(|p| match node {
            "filter" => Some(&p.filter),
            "summary" => Some(&p.summary),
            "digest" => Some(&p.digest),
            "output" => Some(&p.output),
            _ => None,
        })
        .map(|id| vec![id.clone()])
        .unwrap_or_default()
}

pub(super) fn targets(c: &Conversation, event: FlowEvent) -> Vec<String> {
    EDGES
        .iter()
        .filter(|edge| edge.route == Some(event))
        .flat_map(|edge| sessions(c, edge.destination))
        .collect()
}
pub(super) fn retain_delivery_evidence(c: &mut Conversation) {
    for observation in &c.observations {
        let Some(receipt) = &observation.delivery else {
            continue;
        };
        let Some(activity) = c
            .activity
            .iter_mut()
            .find(|a| a.id == observation.invocation_id)
        else {
            continue;
        };
        let delivery = ActivityDelivery {
            session_id: observation.return_session_id.clone(),
            receipt: receipt.clone(),
        };
        if let Some(existing) = activity
            .deliveries
            .iter_mut()
            .find(|d| d.session_id == delivery.session_id)
        {
            *existing = delivery;
        } else {
            activity.deliveries.push(delivery);
        }
    }
}
pub(super) fn release_event(blocks: &[SpokenInput], settings: &VoiceWorkflowSettings) -> FlowEvent {
    if settings.direct_reactions
        && !blocks.is_empty()
        && blocks
            .iter()
            .all(|b| b.reaction.as_deref() == Some("reaction_request"))
    {
        FlowEvent::ReactionReady
    } else {
        FlowEvent::WorkReady
    }
}

fn completion_sound(state: &VoiceState, session: &str) -> Option<bool> {
    let role = state.conversation.as_ref()?.role(session)?;
    let node = if role == "worker" { "workers" } else { role };
    Some(
        *state
            .settings
            .workflow
            .notification_sounds
            .get(node)
            .unwrap_or(&false),
    )
}
impl super::VoiceSessionApplication {
    pub(crate) fn completion_sound_for_session(&self, session: &str) -> Option<bool> {
        let state = self.snapshot().ok()?;
        completion_sound(&state, session)
    }
}

pub(super) fn evidence(
    c: &Conversation,
    source: &str,
    kind: &str,
    targets: &[String],
) -> ExtensionRouteEvidence {
    let source_node = if kind == "accepted_input" {
        "buffer"
    } else {
        match c.role(source) {
            Some("worker") => "workers",
            Some(role) => role,
            None => "external",
        }
    };
    let destinations: Vec<_> = targets
        .iter()
        .map(|session| ExtensionRouteDestination {
            node_id: match c.role(session) {
                Some("worker") => "workers",
                Some(role) => role,
                None => "external",
            }
            .into(),
            session_id: session.clone(),
        })
        .collect();
    // Only name graph connections with a matching event; unknown/dynamic routes remain explicit destinations.
    let event = match kind {
        "accepted_input" if destinations.len() == 1 && destinations[0].node_id == "output" => {
            "reaction_ready"
        }
        "accepted_input" => "work_ready",
        "output_request" => "output_requested",
        "reaction_rerouted" => "reaction_rerouted",
        "session_result" if source_node == "summary" => "summary_result",
        "session_result" if source_node == "router" => "router_result",
        "session_result" => "worker_result",
        "progress" => "progress",
        _ => kind,
    };
    let connection_ids = EDGES
        .iter()
        .filter(|edge| {
            edge.source == source_node
                && destinations.iter().any(|d| d.node_id == edge.destination)
                && (edge.event == event
                    || (source_node == "workers"
                        && edge.destination == "digest"
                        && event == "worker_result"))
        })
        .map(|e| e.id.to_string())
        .collect();
    ExtensionRouteEvidence {
        source_node_id: source_node.into(),
        connection_ids,
        destinations,
    }
}

pub(super) fn view(state: &VoiceState) -> Value {
    let specs = [
        (
            "transcript",
            "Transcript window",
            ExtensionNodeKind::Function,
            "Continuous audio produces revisable, timed sentences.",
            40,
            40,
            vec![],
        ),
        (
            "filter",
            "JEV evaluation",
            ExtensionNodeKind::Capability,
            "Type-safe relevance, completion, continuation and reaction questions.",
            340,
            40,
            vec!["relevance", "completion", "continuation", "reaction"],
        ),
        (
            "buffer",
            "Request buffer",
            ExtensionNodeKind::Function,
            "Retains relevant speech and releases a complete request.",
            640,
            40,
            vec![],
        ),
        (
            "summary",
            "Quick receipt",
            ExtensionNodeKind::Agent,
            "Acknowledges receipt and summarizes while routing proceeds.",
            940,
            210,
            vec!["summary"],
        ),
        (
            "router",
            "Work router",
            ExtensionNodeKind::Agent,
            "Maintains task context and delegates work.",
            640,
            390,
            vec!["router"],
        ),
        (
            "workers",
            "Independent workers",
            ExtensionNodeKind::ExternalSessions,
            "Existing or new sessions execute delegated tasks.",
            340,
            390,
            vec![],
        ),
        (
            "digest",
            "Digest",
            ExtensionNodeKind::Agent,
            "Observes activity and decides when an output is useful.",
            940,
            390,
            vec!["digest"],
        ),
        (
            "output",
            "Output",
            ExtensionNodeKind::Agent,
            "Drafts a conversational response and publishes speech.",
            1240,
            210,
            vec!["output"],
        ),
    ];
    let nodes: Vec<_> = specs
        .into_iter()
        .map(
            |(id, name, kind, description, x, y, prompt_keys)| ExtensionWorkflowNode {
                id,
                name,
                kind,
                description,
                x,
                y,
                prompt_keys,
                notifications: crate::agent_sessions::notifications::NodeNotifications {
                    turn_completion_sound: *state
                        .settings
                        .workflow
                        .notification_sounds
                        .get(id)
                        .unwrap_or(&false),
                },
                session_ids: state
                    .conversation
                    .as_ref()
                    .map(|c| sessions(c, id))
                    .unwrap_or_default(),
            },
        )
        .collect();
    let connections: Vec<_> = EDGES
        .iter()
        .map(|e| ExtensionWorkflowConnection {
            id: e.id,
            name: e.name,
            source: e.source,
            destination: e.destination,
            event: e.event,
            condition: e.condition,
            payload: e.payload,
            enabled: e.route != Some(FlowEvent::ReactionReady)
                || state.settings.workflow.direct_reactions,
        })
        .collect();
    json!({"id":"voice-interaction","owner":"voice","name":"Voice interaction","nodes":nodes,"connections":connections,
        "runtime":{"id":"voice-runtime","name":"Voice runtime","services":["Phone gateway","Continuous transcription","Speech delivery"]}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_sessions::domain::PipelineSessions;
    fn conversation() -> Conversation {
        Conversation {
            main_session_id: "router-session".into(),
            pipeline: Some(PipelineSessions {
                filter: "jev-session".into(),
                summary: "receipt-session".into(),
                digest: "digest-session".into(),
                output: "output-session".into(),
            }),
            ..Default::default()
        }
    }
    #[test]
    fn release_routes_and_display_share_the_same_connections() {
        let c = conversation();
        let reaction = SpokenInput {
            reaction: Some("reaction_request".into()),
            ..Default::default()
        };
        assert_eq!(
            targets(
                &c,
                release_event(&[reaction.clone()], &VoiceWorkflowSettings::default())
            ),
            vec!["output-session"]
        );
        assert_eq!(
            targets(
                &c,
                release_event(
                    &[reaction.clone(), SpokenInput::default()],
                    &VoiceWorkflowSettings::default()
                )
            ),
            vec!["receipt-session", "router-session", "digest-session"]
        );
        assert_eq!(
            targets(
                &c,
                release_event(
                    &[reaction],
                    &VoiceWorkflowSettings {
                        direct_reactions: false,
                        ..Default::default()
                    }
                )
            ),
            vec!["receipt-session", "router-session", "digest-session"]
        );
        let evidence = evidence(
            &c,
            "user",
            "accepted_input",
            &targets(&c, FlowEvent::WorkReady),
        );
        assert_eq!(
            evidence.connection_ids,
            vec!["buffer-summary", "buffer-router", "buffer-digest"]
        );
        let definition = view(&VoiceState {
            conversation: Some(c),
            ..Default::default()
        });
        for id in evidence.connection_ids {
            assert!(definition["connections"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["id"] == id));
        }
    }
    #[test]
    fn output_and_summary_bind_to_existing_sessions() {
        let c = conversation();
        assert_eq!(
            targets(&c, FlowEvent::SummaryResult),
            vec!["router-session", "digest-session"]
        );
        assert_eq!(
            targets(&c, FlowEvent::OutputRequested),
            vec!["output-session"]
        );
        assert_eq!(
            targets(&c, FlowEvent::ReactionRerouted),
            vec!["router-session"]
        );
        assert_eq!(
            evidence(
                &c,
                "output-session",
                "reaction_rerouted",
                &targets(&c, FlowEvent::ReactionRerouted)
            )
            .connection_ids,
            vec!["output-router"]
        );
        assert!(
            evidence(&c, "unknown", "custom", &["unbound-session".into()])
                .connection_ids
                .is_empty()
        );
    }
    #[test]
    fn voice_nodes_are_silent_by_default_and_one_node_can_be_enabled() {
        let mut state = VoiceState {
            conversation: Some(conversation()),
            ..Default::default()
        };
        for id in [
            "router-session",
            "receipt-session",
            "jev-session",
            "digest-session",
            "output-session",
        ] {
            assert_eq!(completion_sound(&state, id), Some(false));
        }
        assert_eq!(completion_sound(&state, "unrelated-session"), None);
        assert!(view(&state)["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["notifications"]["turnCompletionSound"] == false));
        state
            .settings
            .workflow
            .notification_sounds
            .insert("digest".into(), true);
        assert_eq!(completion_sound(&state, "digest-session"), Some(true));
        assert_eq!(completion_sound(&state, "router-session"), Some(false));
    }
    #[test]
    fn accepted_delivery_evidence_survives_removal_from_the_pending_queue() {
        use crate::agent_sessions::application::ProductDeliveryReceipt;
        use crate::voice_sessions::domain::{Activity, Observation};
        let mut c = conversation();
        c.activity.push(serde_json::from_value::<Activity>(json!({"id":"activity","source":"user","kind":"accepted_input","createdAt":"now","data":{}})).unwrap());
        c.observations.push(Observation {
            session_id: "user".into(),
            invocation_id: "activity".into(),
            return_session_id: "output-session".into(),
            detail: json!({}),
            created_at: "now".into(),
            delivery: Some(ProductDeliveryReceipt {
                session_id: "output-session".into(),
                invocation_id: "turn".into(),
                input_id: Some("steer".into()),
                state: "queued".into(),
            }),
        });
        retain_delivery_evidence(&mut c);
        assert_eq!(c.activity[0].deliveries[0].receipt.state, "queued");
        c.observations[0].delivery.as_mut().unwrap().state = "accepted".into();
        retain_delivery_evidence(&mut c);
        c.observations.clear();
        assert_eq!(c.activity[0].deliveries.len(), 1);
        assert_eq!(c.activity[0].deliveries[0].receipt.state, "accepted");
        assert!(c.activity[0].route.is_none());
    }
}
