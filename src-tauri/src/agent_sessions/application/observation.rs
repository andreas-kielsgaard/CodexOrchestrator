use crate::agent_sessions::{
    domain::{
        AgentInvocation, AgentInvocationStatus, AgentRuntimeEvent, AgentRuntimeEventId,
        AgentRuntimeEventSource, ExternalRuntimeContextId, NormalizedRuntimeEventKind,
        NormalizedToolActivity,
    },
    ports::AgentInvocationHistory,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeObservationCorrelation {
    pub(crate) event_id: AgentRuntimeEventId,
    pub(crate) sequence: u64,
    pub(crate) recorded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalContextObservation {
    pub(crate) external_context_id: ExternalRuntimeContextId,
    pub(crate) correlation: RuntimeObservationCorrelation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderTerminalStatus {
    Completed,
    Failed,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderTerminalObservation {
    pub(crate) status: ProviderTerminalStatus,
    pub(crate) correlation: RuntimeObservationCorrelation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessTerminalObservation {
    pub(crate) status: AgentInvocationStatus,
    pub(crate) completed_at: DateTime<Utc>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SemanticMcpToolObservation {
    pub(crate) activity: NormalizedToolActivity,
    pub(crate) correlation: RuntimeObservationCorrelation,
}

/// Rebuilt from one invocation's durable records. Application acceptance remains consumer-owned
/// and intentionally has no field here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInvocationObservation {
    pub(crate) launch_accepted_at: Option<DateTime<Utc>>,
    pub(crate) external_context: Option<ExternalContextObservation>,
    pub(crate) provider_activity: Option<RuntimeObservationCorrelation>,
    pub(crate) provider_terminal: Option<ProviderTerminalObservation>,
    pub(crate) process_terminal: Option<ProcessTerminalObservation>,
    pub(crate) mcp_tool_activities: Vec<SemanticMcpToolObservation>,
    /// Old records can identify MCP activity but lack newer typed fields; raw payload is not
    /// reparsed as a migration.
    pub(crate) mcp_tool_activity_partial: bool,
}

pub(crate) fn project_invocation_observation(
    history: &AgentInvocationHistory,
) -> AgentInvocationObservation {
    let mut external_context = None;
    let mut provider_activity = None;
    let mut observed_provider_terminal = None;
    let mut mcp_tool_activities = Vec::new();
    let mut mcp_tool_activity_partial = false;
    for event in &history.events {
        let Some(normalized) = event.normalized.as_ref() else {
            continue;
        };
        let correlation = correlation(event);
        if external_context.is_none()
            && normalized.kind == NormalizedRuntimeEventKind::RuntimeContextEstablished
        {
            if let Some(external_context_id) = normalized.external_context_id.clone() {
                external_context = Some(ExternalContextObservation {
                    external_context_id,
                    correlation: correlation.clone(),
                });
            }
        }
        if provider_activity.is_none() && is_provider_activity(event) {
            provider_activity = Some(correlation.clone());
        }
        if observed_provider_terminal.is_none() {
            observed_provider_terminal = provider_terminal(normalized, correlation.clone());
        }
        if let Some(activity) = normalized.tool_activity.clone() {
            mcp_tool_activities.push(SemanticMcpToolObservation {
                activity,
                correlation,
            });
        } else if normalized.kind == NormalizedRuntimeEventKind::ToolActivity
            && normalized
                .details
                .as_ref()
                .and_then(|details| details.get("itemType"))
                .and_then(serde_json::Value::as_str)
                == Some("mcp_tool_call")
        {
            mcp_tool_activity_partial = true;
        }
    }
    AgentInvocationObservation {
        launch_accepted_at: history.launch_accepted_at,
        external_context,
        provider_activity,
        provider_terminal: observed_provider_terminal,
        process_terminal: process_terminal(&history.invocation),
        mcp_tool_activities,
        mcp_tool_activity_partial,
    }
}

fn correlation(event: &AgentRuntimeEvent) -> RuntimeObservationCorrelation {
    RuntimeObservationCorrelation {
        event_id: event.id.clone(),
        sequence: event.sequence,
        recorded_at: event.recorded_at,
    }
}

fn is_provider_activity(event: &AgentRuntimeEvent) -> bool {
    let Some(normalized) = event.normalized.as_ref() else {
        return false;
    };
    normalized.kind == NormalizedRuntimeEventKind::ProcessingStarted
        || (event.source == AgentRuntimeEventSource::Stdout
            && !matches!(
                normalized.kind,
                NormalizedRuntimeEventKind::RuntimeContextEstablished
                    | NormalizedRuntimeEventKind::Unknown
            ))
}

fn provider_terminal(
    normalized: &crate::agent_sessions::domain::NormalizedRuntimeEvent,
    correlation: RuntimeObservationCorrelation,
) -> Option<ProviderTerminalObservation> {
    let status = match normalized.kind {
        NormalizedRuntimeEventKind::InvocationCompleted => ProviderTerminalStatus::Completed,
        NormalizedRuntimeEventKind::RuntimeError => match normalized
            .details
            .as_ref()
            .and_then(|details| details.get("providerTerminal"))
            .and_then(serde_json::Value::as_str)
        {
            Some("failed") => ProviderTerminalStatus::Failed,
            Some("error") => ProviderTerminalStatus::Error,
            _ => return None,
        },
        _ => return None,
    };
    Some(ProviderTerminalObservation {
        status,
        correlation,
    })
}

fn process_terminal(invocation: &AgentInvocation) -> Option<ProcessTerminalObservation> {
    invocation
        .status
        .is_terminal()
        .then(|| ProcessTerminalObservation {
            status: invocation.status,
            completed_at: invocation
                .completed_at
                .expect("terminal invocation requires completion time"),
            exit_code: invocation.exit_code,
            signal: invocation.signal.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::domain::{
        AgentInvocationInputProvenance, AgentRuntimeEventSource, AgentRuntimeOptions,
        AgentSessionId, NormalizedRuntimeEvent, NormalizedToolActivity, ToolActivityPhase,
        ToolResultClassification,
    };
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    #[test]
    fn projects_truthful_invocation_states_without_collapsing_shared_contexts() {
        let launch_only = history("launch", AgentInvocationStatus::Pending, Some(1), vec![]);
        let launch = project_invocation_observation(&launch_only);
        assert_eq!(launch.launch_accepted_at, Some(at(1)));
        assert!(launch.external_context.is_none());
        assert!(launch.provider_activity.is_none());
        assert!(launch.process_terminal.is_none());

        let context_only = history(
            "context",
            AgentInvocationStatus::Completed,
            None,
            vec![context_event("context-event", "shared", 4)],
        );
        let context = project_invocation_observation(&context_only);
        assert_eq!(
            context
                .external_context
                .unwrap()
                .external_context_id
                .as_str(),
            "shared"
        );
        assert!(context.provider_activity.is_none());
        assert!(context.provider_terminal.is_none());
        assert_eq!(
            context.process_terminal.unwrap().status,
            AgentInvocationStatus::Completed
        );

        let activity_terminal = history(
            "activity",
            AgentInvocationStatus::Failed,
            None,
            vec![
                processing_event("activity-event", 5),
                terminal_event("provider-failed", 6, ProviderTerminalStatus::Failed),
            ],
        );
        let activity = project_invocation_observation(&activity_terminal);
        assert_eq!(activity.provider_activity.unwrap().sequence, 5);
        assert_eq!(
            activity.provider_terminal.unwrap().status,
            ProviderTerminalStatus::Failed
        );
        assert_eq!(
            activity.process_terminal.unwrap().status,
            AgentInvocationStatus::Failed
        );

        let first = history(
            "first",
            AgentInvocationStatus::Completed,
            None,
            vec![context_event("first-context", "shared", 7)],
        );
        let second = history(
            "second",
            AgentInvocationStatus::Completed,
            None,
            vec![context_event("second-context", "shared", 9)],
        );
        let first = project_invocation_observation(&first)
            .external_context
            .unwrap();
        let second = project_invocation_observation(&second)
            .external_context
            .unwrap();
        assert_eq!(first.external_context_id, second.external_context_id);
        assert_ne!(first.correlation.event_id, second.correlation.event_id);
    }

    #[test]
    fn projects_typed_and_historical_mcp_evidence_without_raw_reparsing() {
        let typed = history(
            "typed",
            AgentInvocationStatus::Completed,
            None,
            vec![
                mcp_event("mcp-success", 1, ToolResultClassification::Succeeded),
                mcp_event("mcp-failed", 2, ToolResultClassification::Failed),
                mcp_event("mcp-unknown", 3, ToolResultClassification::Unknown),
            ],
        );
        let observed = project_invocation_observation(&typed);
        assert_eq!(observed.mcp_tool_activities.len(), 3);
        assert_eq!(
            observed.mcp_tool_activities[0].activity.phase,
            ToolActivityPhase::Completed
        );
        assert_eq!(
            observed.mcp_tool_activities[1]
                .activity
                .result_classification,
            ToolResultClassification::Failed
        );
        assert_eq!(
            observed.mcp_tool_activities[2]
                .activity
                .result_classification,
            ToolResultClassification::Unknown
        );
        assert!(!observed.mcp_tool_activity_partial);

        let old = history(
            "old",
            AgentInvocationStatus::Completed,
            None,
            vec![event(
                "old-mcp",
                1,
                NormalizedRuntimeEventKind::ToolActivity,
                Some(json!({"itemType":"mcp_tool_call"})),
                None,
            )],
        );
        let partial = project_invocation_observation(&old);
        assert!(partial.mcp_tool_activities.is_empty());
        assert!(partial.mcp_tool_activity_partial);
    }

    fn history(
        id: &str,
        status: AgentInvocationStatus,
        launch_accepted_at: Option<i64>,
        events: Vec<AgentRuntimeEvent>,
    ) -> AgentInvocationHistory {
        AgentInvocationHistory {
            invocation: AgentInvocation {
                id: crate::agent_sessions::domain::AgentInvocationId::new(id).unwrap(),
                session_id: AgentSessionId::new("session").unwrap(),
                submitted_text: "work".into(),
                input_provenance: AgentInvocationInputProvenance::User,
                status,
                requested_options: AgentRuntimeOptions::default(),
                effective_options: (status != AgentInvocationStatus::Pending)
                    .then(AgentRuntimeOptions::default),
                started_at: (status != AgentInvocationStatus::Pending).then(|| at(0)),
                completed_at: status.is_terminal().then(|| at(2)),
                exit_code: (status == AgentInvocationStatus::Completed).then_some(0),
                signal: None,
                runtime_error: None,
                diagnostics: vec![],
                created_at: at(0),
                updated_at: at(2),
            },
            launch_accepted_at: launch_accepted_at.map(at),
            events,
        }
    }
    fn event(
        id: &str,
        sequence: u64,
        kind: NormalizedRuntimeEventKind,
        details: Option<serde_json::Value>,
        tool_activity: Option<NormalizedToolActivity>,
    ) -> AgentRuntimeEvent {
        AgentRuntimeEvent {
            id: AgentRuntimeEventId::new(id).unwrap(),
            invocation_id: crate::agent_sessions::domain::AgentInvocationId::new("event-owner")
                .unwrap(),
            sequence,
            source: AgentRuntimeEventSource::Stdout,
            raw_payload: json!({"opaque":true}),
            normalized: Some(NormalizedRuntimeEvent {
                kind,
                text: None,
                external_context_id: None,
                usage: None,
                details,
                tool_activity,
            }),
            recorded_at: at(sequence as i64),
        }
    }
    fn context_event(id: &str, context: &str, sequence: u64) -> AgentRuntimeEvent {
        let mut event = event(
            id,
            sequence,
            NormalizedRuntimeEventKind::RuntimeContextEstablished,
            None,
            None,
        );
        event.normalized.as_mut().unwrap().external_context_id =
            Some(ExternalRuntimeContextId::new(context).unwrap());
        event
    }
    fn processing_event(id: &str, sequence: u64) -> AgentRuntimeEvent {
        event(
            id,
            sequence,
            NormalizedRuntimeEventKind::ProcessingStarted,
            None,
            None,
        )
    }
    fn terminal_event(
        id: &str,
        sequence: u64,
        status: ProviderTerminalStatus,
    ) -> AgentRuntimeEvent {
        event(
            id,
            sequence,
            NormalizedRuntimeEventKind::RuntimeError,
            Some(
                json!({"providerTerminal": match status { ProviderTerminalStatus::Failed => "failed", ProviderTerminalStatus::Error => "error", ProviderTerminalStatus::Completed => "completed" }}),
            ),
            None,
        )
    }
    fn mcp_event(
        id: &str,
        sequence: u64,
        result_classification: ToolResultClassification,
    ) -> AgentRuntimeEvent {
        event(
            id,
            sequence,
            NormalizedRuntimeEventKind::ToolActivity,
            Some(json!({"itemType":"mcp_tool_call"})),
            Some(NormalizedToolActivity {
                phase: ToolActivityPhase::Completed,
                item_id: Some(id.into()),
                server: Some("orchestration".into()),
                tool: Some("submit_epic_plan_proposal".into()),
                status: None,
                result_classification,
            }),
        )
    }
    fn at(second: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + second, 0)
            .single()
            .unwrap()
    }
}
