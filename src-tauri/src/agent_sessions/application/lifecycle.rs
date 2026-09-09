//! Cancellation, restart reconciliation and runtime continuity.

use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionNotification,
    CancelAgentInvocationCommand,
};
use crate::agent_sessions::domain::{
    AgentDiagnosticSource, AgentInvocation, AgentInvocationTerminalStatus, AgentRuntimeFailure,
    AgentSession, InvocationCompletion,
};
use crate::agent_sessions::ports::{
    ListAgentSessionsQuery, RuntimeInvocationOutcome, RuntimeUpdate,
};
use chrono::{DateTime, Utc};

impl AgentSessionApplication {
    pub(crate) fn cancel_invocation(
        &self,
        command: CancelAgentInvocationCommand,
    ) -> Result<AgentInvocation, AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        if !invocation.status.is_active() {
            return Ok(invocation);
        }
        if let Err(error) = self.runtime.cancel_invocation(&command.invocation_id) {
            self.record_diagnostic(
                &command.invocation_id,
                AgentDiagnosticSource::Runtime,
                "runtime_cancellation_failed",
                error.message.clone(),
                serde_json::to_value(&error).ok(),
            );
            return Err(AgentSessionApplicationError::runtime(error));
        }
        self.repository
            .get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))
    }

    pub(crate) fn reconcile_startup(&self) -> Result<usize, AgentSessionApplicationError> {
        let mut reconciled = 0;
        for summary in self
            .repository
            .list_session_summaries(ListAgentSessionsQuery::default())
            .map_err(AgentSessionApplicationError::repository)?
        {
            let history = self
                .repository
                .load_session_history(&summary.session.id)
                .map_err(AgentSessionApplicationError::repository)?
                .ok_or_else(|| {
                    AgentSessionApplicationError::not_found(
                        "Agent Session disappeared during startup reconciliation",
                    )
                })?;
            for invocation_history in history.invocations {
                let invocation = invocation_history.invocation;
                if !invocation.status.is_active() {
                    continue;
                }
                if let Some((outcome, completed_at)) = known_terminal_delivery(&invocation) {
                    let updated = self
                        .repository
                        .finish_invocation(
                            &invocation.id,
                            InvocationCompletion {
                                status: outcome.status,
                                completed_at,
                                exit_code: outcome.exit_code,
                                signal: outcome.signal,
                                runtime_error: outcome.runtime_error,
                            },
                            completed_at,
                        )
                        .map_err(AgentSessionApplicationError::repository)?;
                    self.update_lanes.remove_invocation(&invocation.id);
                    reconciled += 1;
                    self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                        session_id: history.session.id.clone(),
                        invocation: updated,
                    });
                    continue;
                }
                let completed_at = self.clock.now();
                let runtime_error = if self
                    .repository
                    .invocation_launch_accepted_at(&invocation.id)
                    .map_err(AgentSessionApplicationError::repository)?
                    .is_none()
                {
                    Some(AgentRuntimeFailure {
                        code: "runtime_startup_without_launch_acceptance".to_string(),
                        message: "application restarted without durable launch acceptance"
                            .to_string(),
                        details: None,
                    })
                } else {
                    None
                };
                let updated = self
                    .repository
                    .finish_invocation(
                        &invocation.id,
                        InvocationCompletion {
                            status: AgentInvocationTerminalStatus::Interrupted,
                            completed_at,
                            exit_code: None,
                            signal: None,
                            runtime_error,
                        },
                        completed_at,
                    )
                    .map_err(AgentSessionApplicationError::repository)?;
                self.update_lanes.remove_invocation(&invocation.id);
                reconciled += 1;
                self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                    session_id: history.session.id.clone(),
                    invocation: updated,
                });
            }
        }
        Ok(reconciled)
    }

    pub(crate) fn shutdown_runtime(&self) -> Result<(), AgentSessionApplicationError> {
        self.runtime
            .shutdown()
            .map_err(AgentSessionApplicationError::runtime)
    }

    pub(super) fn repair_missing_runtime_binding(
        &self,
        session: AgentSession,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        if session.runtime_binding.external_context_id.is_some() {
            return Ok(session);
        }
        let history = self
            .repository
            .load_session_history(&session.id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        let mut recovered = None;
        for event in history
            .invocations
            .iter()
            .flat_map(|history| history.events.iter())
        {
            let Some(external_context_id) = event
                .normalized
                .as_ref()
                .filter(|normalized| {
                    normalized.kind
                        == crate::agent_sessions::domain::NormalizedRuntimeEventKind::RuntimeContextEstablished
                })
                .and_then(|normalized| normalized.external_context_id.clone())
            else {
                continue;
            };
            if recovered
                .as_ref()
                .is_some_and(|current| current != &external_context_id)
            {
                return Err(AgentSessionApplicationError::conflict(
                    "durable runtime context evidence contains conflicting external identities",
                ));
            }
            recovered = Some(external_context_id);
        }
        let Some(external_context_id) = recovered else {
            return Ok(session);
        };
        let mut binding = session.runtime_binding.clone();
        binding.external_context_id = Some(external_context_id);
        self.repository
            .update_runtime_binding(&session.id, binding, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)
    }
}

fn known_terminal_delivery(
    invocation: &AgentInvocation,
) -> Option<(RuntimeInvocationOutcome, DateTime<Utc>)> {
    invocation.diagnostics.iter().rev().find_map(|diagnostic| {
        if diagnostic.source != AgentDiagnosticSource::Repository
            || diagnostic.code != "runtime_update_delivery_failed"
            || diagnostic.recorded_at < invocation.created_at
            || invocation
                .started_at
                .is_some_and(|started_at| diagnostic.recorded_at < started_at)
        {
            return None;
        }
        let failed_update = diagnostic.details.as_ref()?.get("failedUpdate")?.clone();
        match serde_json::from_value::<RuntimeUpdate>(failed_update).ok()? {
            RuntimeUpdate::Finished(outcome) => {
                invocation
                    .finish(
                        InvocationCompletion {
                            status: outcome.status,
                            completed_at: diagnostic.recorded_at,
                            exit_code: outcome.exit_code,
                            signal: outcome.signal.clone(),
                            runtime_error: outcome.runtime_error.clone(),
                        },
                        diagnostic.recorded_at,
                    )
                    .ok()?;
                Some((outcome, diagnostic.recorded_at))
            }
            RuntimeUpdate::Event(_) => None,
        }
    })
}
