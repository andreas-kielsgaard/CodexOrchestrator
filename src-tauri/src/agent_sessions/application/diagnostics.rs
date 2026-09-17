//! Persistence and delivery of application diagnostics.

use super::{AgentSessionApplication, AgentSessionNotification};
use crate::agent_sessions::domain::{
    AgentDiagnostic, AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocationId,
    AgentSessionId,
};
use serde_json::{json, Value};

impl AgentSessionApplication {
    pub(super) fn notify_or_record(&self, notification: AgentSessionNotification) {
        let (invocation_id, session_id) = notification_ids(&notification);
        if let Err(error) = self.notifier.notify(notification) {
            self.record_diagnostic(
                &invocation_id,
                AgentDiagnosticSource::Transport,
                "agent_session_notification_failed",
                error,
                Some(json!({"sessionId": session_id})),
            );
        }
    }

    pub(super) fn record_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        source: AgentDiagnosticSource,
        code: &str,
        message: String,
        details: Option<Value>,
    ) {
        let diagnostic = AgentDiagnostic {
            source,
            severity: AgentDiagnosticSeverity::Error,
            code: code.to_string(),
            message,
            details,
            recorded_at: self.clock.now(),
        };
        let Ok(invocation) = self
            .repository
            .append_invocation_diagnostic(invocation_id, diagnostic)
        else {
            return;
        };
        let _ = self
            .notifier
            .notify(AgentSessionNotification::DiagnosticRecorded {
                session_id: invocation.session_id.clone(),
                invocation,
            });
    }
}

fn notification_ids(
    notification: &AgentSessionNotification,
) -> (AgentInvocationId, AgentSessionId) {
    match notification {
        AgentSessionNotification::TargetTransitionUpdated { .. } => {
            unreachable!("target-transition notifications do not record invocation diagnostics")
        }
        AgentSessionNotification::PreparationUpdated {
            session_id,
            invocation_id,
        } => (invocation_id.clone(), session_id.clone()),
        AgentSessionNotification::SteeringAccepted {
            session_id,
            invocation_id,
            ..
        } => (invocation_id.clone(), session_id.clone()),
        AgentSessionNotification::EventPersisted { session_id, event } => {
            (event.invocation_id.clone(), session_id.clone())
        }
        AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        }
        | AgentSessionNotification::DiagnosticRecorded {
            session_id,
            invocation,
        } => (invocation.id.clone(), session_id.clone()),
    }
}
