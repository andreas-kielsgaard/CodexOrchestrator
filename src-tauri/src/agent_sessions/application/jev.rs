use super::*;
use crate::{
    agent_sessions::{
        domain::{
            AgentInvocation, AgentInvocationId, AgentInvocationInputProvenance, AgentSession,
            AgentSessionId,
        },
        ports::InvocationContent,
    },
    execution_configuration::{
        CapabilityProfile, NodeProfile, SessionCreationRequest, SessionProfileResolver,
    },
    execution_targets::domain::ExecutionBinding,
    runtime::providers::jev::{
        contracts::{JevAnswer, JevEvaluation, JevResult},
        DEFAULT_MODEL, PROVIDER,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateJevApiSessionInput {
    pub(crate) title: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JevApiInvocationView {
    pub(crate) invocation: AgentInvocation,
    pub(crate) evaluation: JevEvaluation,
    pub(crate) result: Option<JevResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JevQuerySummary {
    invocation_id: AgentInvocationId,
    status: super::super::domain::AgentInvocationStatus,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    input_preview: String,
    question_count: usize,
    answers: Vec<String>,
}

impl AgentSessionApplication {
    pub(crate) fn create_jev_api_session(
        &self,
        input: CreateJevApiSessionInput,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        let model = input.model.unwrap_or_else(|| DEFAULT_MODEL.into());
        let source = self
            .configuration_source(PROVIDER)
            .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        let runtime = source
            .profile_for_configuration(&model, None)
            .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        let capability = CapabilityProfile {
            contract_version: 1,
            capability_profile_id: "jev-api".into(),
            name: "Jev API".into(),
            revision: 1,
            execution: ExecutionBinding {
                provider: PROVIDER.into(),
                configuration_ref: model.clone(),
                ..Default::default()
            },
            defaults: runtime.locked.clone(),
            route_policies: vec![],
            default_route_id: None,
            allowed_capabilities: runtime.exposure.clone(),
        };
        let resolution = SessionProfileResolver::resolve_snapshot(
            runtime.clone(),
            SessionCreationRequest {
                contract_version: 1,
                capability_profile: capability,
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: runtime.exposure,
                    pinned_defaults: runtime.locked,
                },
                agent_mcp_configuration: Default::default(),
                session_skill_inputs: vec![],
            },
        )
        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        self.create_session_with_ownership(
            CreateAgentSessionCommand {
                title: input.title.or_else(|| Some("Jev API".into())),
                working_directory: None,
                requested_options: runtime_options(resolution.session_profile().pinned_defaults()),
            },
            AgentSessionOwnership {
                session_profile: Some(resolution),
                ..Default::default()
            },
        )
    }

    /// Serializes provider input inside the adapter boundary; the existing invocation retains it.
    /// Reusing an invocation ID returns its previous acknowledgement without another API call.
    pub(crate) fn invoke_jev_api_session(
        &self,
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        evaluation: JevEvaluation,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        evaluation
            .validate()
            .map_err(AgentSessionApplicationError::invalid)?;
        let session = self.load_session(&session_id)?.session;
        self.require_jev_session(&session)?;
        let query = serde_json::to_string(&evaluation).map_err(|_| {
            AgentSessionApplicationError::invalid("Unable to encode Jev evaluation")
        })?;
        self.delivery().deliver(InvocationDeliveryIntent {
            invocation_id: Some(invocation_id),
            provenance: AgentInvocationInputProvenance::Application,
            session_id: Some(session_id),
            title: None,
            working_directory: None,
            requested_options: None,
            content: InvocationContent::query(query),
            attachments: vec![],
            launch_extension: None,
            prepared_only: false,
        })
    }

    pub(crate) fn load_jev_api_invocation(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
    ) -> Result<JevApiInvocationView, AgentSessionApplicationError> {
        let history = self.load_session(session_id)?;
        self.require_jev_session(&history.session)?;
        let invocation = history
            .invocations
            .into_iter()
            .find(|entry| entry.invocation.id == *invocation_id)
            .ok_or_else(|| {
                AgentSessionApplicationError::not_found("Jev invocation not found in this session")
            })?;
        let result = invocation
            .events
            .iter()
            .filter_map(|event| event.normalized.as_ref())
            .filter(|event| event.agent_message_role() == Some("final"))
            .filter_map(|event| event.details.as_ref()?.get("structuredOutput"))
            .last()
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|_| AgentSessionApplicationError::invalid("Stored Jev result is invalid"))?;
        let evaluation =
            serde_json::from_str(&invocation.invocation.submitted_text).map_err(|_| {
                AgentSessionApplicationError::invalid("Stored Jev evaluation is invalid")
            })?;
        Ok(JevApiInvocationView {
            invocation: invocation.invocation,
            evaluation,
            result,
        })
    }

    pub(crate) fn list_jev_api_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<JevQuerySummary>, AgentSessionApplicationError> {
        let history = self.load_session(session_id)?;
        self.require_jev_session(&history.session)?;
        history
            .invocations
            .into_iter()
            .rev()
            .map(|entry| {
                let evaluation: JevEvaluation =
                    serde_json::from_str(&entry.invocation.submitted_text).map_err(|_| {
                        AgentSessionApplicationError::invalid("Stored Jev evaluation is invalid")
                    })?;
                let result = entry
                    .events
                    .iter()
                    .filter_map(|event| event.normalized.as_ref())
                    .filter_map(|event| event.details.as_ref()?.get("structuredOutput"))
                    .last()
                    .map(|value| serde_json::from_value::<JevResult>(value.clone()))
                    .transpose()
                    .map_err(|_| {
                        AgentSessionApplicationError::invalid("Stored Jev result is invalid")
                    })?;
                let preview = evaluation
                    .state
                    .get("recentSpeech")
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.last())
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| evaluation.state.to_string());
                Ok(JevQuerySummary {
                    invocation_id: entry.invocation.id,
                    status: entry.invocation.status,
                    created_at: entry.invocation.created_at,
                    started_at: entry.invocation.started_at,
                    completed_at: entry.invocation.completed_at,
                    input_preview: preview.chars().take(160).collect(),
                    question_count: evaluation.questions.len(),
                    answers: result
                        .map(|r| {
                            r.answers
                                .into_iter()
                                .map(|(key, answer)| {
                                    let kind = key.split(':').next().unwrap_or(&key);
                                    match answer {
                                        JevAnswer::Choice { choice, .. } => {
                                            format!("{kind}: {choice}")
                                        }
                                        JevAnswer::Noul { noul } => format!("{kind}: {noul:.2}"),
                                        JevAnswer::Score { score, .. } => {
                                            format!("{kind}: {score:.2}")
                                        }
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    fn require_jev_session(
        &self,
        session: &AgentSession,
    ) -> Result<(), AgentSessionApplicationError> {
        let pinned = session.session_profile.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("This is not a Jev API session")
        })?;
        pinned
            .verify_digest()
            .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        let profile = pinned.session_profile();
        if profile.configuration().provider != PROVIDER
            || session.execution_target.is_some()
            || session.requested_options != runtime_options(profile.pinned_defaults())
            || session.runtime_binding.external_context_id.is_some()
        {
            return Err(AgentSessionApplicationError::invalid(
                "Jev API session binding does not match its pinned configuration",
            ));
        }
        Ok(())
    }
}
