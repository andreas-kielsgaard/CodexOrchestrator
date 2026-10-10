use super::*;
use crate::agent_sessions::{domain::AgentInvocationInputProvenance, ports::InvocationContent};
use orchid_engine::contracts::InvocationContextPart;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductDeliveryReceipt {
    pub session_id: String,
    pub invocation_id: String,
    pub input_id: Option<String>,
    pub state: String,
}

impl AgentSessionApplication {
    /// Product input shares the ordinary queue and active-turn steering service.
    pub(crate) fn deliver_or_steer_product(
        &self,
        session: crate::agent_sessions::domain::AgentSessionId,
        input_id: String,
        content: InvocationContent,
    ) -> Result<ProductDeliveryReceipt, AgentSessionApplicationError> {
        let history = self.load_session(&session)?;
        if let Some(active) = history.invocations.iter().rev().find(|h| {
            h.invocation.status == crate::agent_sessions::domain::AgentInvocationStatus::Running
        }) {
            let mut text = content.primary_query.clone();
            for part in &content.context {
                if matches!(part, InvocationContextPart::InitialInstructions { .. }) {
                    continue;
                }
                let content = match part {
                    InvocationContextPart::InitialInstructions { content, .. }
                    | InvocationContextPart::ReferencedProductContent { content, .. }
                    | InvocationContextPart::MissedConversationTurn { content, .. }
                    | InvocationContextPart::SkillGuidance { content, .. } => content,
                };
                text.push_str("\n\n");
                text.push_str(content);
            }
            match self.steer_session(SteerAgentSessionCommand {
                session_id: session.clone(),
                invocation_id: active.invocation.id.clone(),
                input_id: format!("{input_id}:{}", active.invocation.id),
                text,
                attachments: vec![],
            }) {
                Ok(interaction) => {
                    return Ok(ProductDeliveryReceipt {
                        session_id: session.to_string(),
                        invocation_id: active.invocation.id.to_string(),
                        input_id: Some(interaction.id),
                        state: interaction.state,
                    })
                }
                Err(error) => {
                    // A finishing turn may cease to be steerable between lookup and submission.
                    if self.load_session(&session)?.invocations.iter().any(|h| {
                        h.invocation.status
                            == crate::agent_sessions::domain::AgentInvocationStatus::Running
                    }) {
                        return Err(error);
                    }
                }
            }
        }
        let result = self.deliver_product_content(
            session,
            AgentInvocationInputProvenance::Application,
            content,
        )?;
        Ok(ProductDeliveryReceipt {
            session_id: result.acknowledgement.session_id.to_string(),
            invocation_id: result.acknowledgement.invocation_id.to_string(),
            input_id: None,
            state: "queued".into(),
        })
    }
}
