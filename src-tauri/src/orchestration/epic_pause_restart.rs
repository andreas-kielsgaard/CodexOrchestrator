//! Durable Epic controls. Launch acceptance is deliberately weaker than provider receipt,
//! instruction compliance, or observed resumed work.
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, AgentSessionNotification, ApplicationInvocationLaunchEvidence,
        CancelAgentInvocationCommand, SendAgentSessionMessageCommand,
        SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentInvocationId, AgentInvocationStatus, AgentSessionId},
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub(crate) const EPIC_PAUSE_RESTART_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS epic_control_actions (
 id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('pause','restart')),
 status TEXT NOT NULL CHECK(status IN ('pending','partial','attention','completed')),
 requested_at TEXT NOT NULL, completed_at TEXT
);
CREATE TABLE IF NOT EXISTS epic_control_targets (
 action_id TEXT NOT NULL, session_id TEXT NOT NULL, source_invocation_id TEXT NOT NULL,
 cancel_requested_at TEXT, interruption_status TEXT NOT NULL CHECK(interruption_status IN ('awaiting_cancel','canceled','interrupted','failed','completed')),
 interruption_observed_at TEXT, message_invocation_id TEXT, message_persisted_at TEXT, launch_accepted_at TEXT,
 failure_category TEXT, failure_detail TEXT,
 PRIMARY KEY(action_id, source_invocation_id),
 FOREIGN KEY(action_id) REFERENCES epic_control_actions(id) ON DELETE RESTRICT,
 FOREIGN KEY(session_id) REFERENCES agent_sessions(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS epic_control_actions_by_epic ON epic_control_actions(epic_id, kind, requested_at);
CREATE INDEX IF NOT EXISTS epic_control_targets_by_source ON epic_control_targets(source_invocation_id);
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpicControlOutcome {
    pub(crate) action_id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) target_count: usize,
    pub(crate) launched_count: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpicControlQuery {
    pub(crate) epic_id: String,
    pub(crate) pause: EpicControlRead,
    pub(crate) restart: EpicControlRead,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpicControlRead {
    pub(crate) availability: String,
    pub(crate) reason: String,
    pub(crate) current: Option<EpicControlOutcome>,
}

pub(crate) struct EpicPauseRestartService {
    connection: Arc<Mutex<Connection>>,
    sessions: Arc<AgentSessionApplication>,
}
impl EpicPauseRestartService {
    pub(crate) fn open(
        path: &Path,
        sessions: Arc<AgentSessionApplication>,
    ) -> Result<Arc<Self>, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        crate::storage::configure_sqlite_connection(&connection).map_err(|e| e.to_string())?;
        connection
            .execute_batch(EPIC_PAUSE_RESTART_SCHEMA)
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            connection: Arc::new(Mutex::new(connection)),
            sessions,
        }))
    }

    pub(crate) fn request(&self, epic_id: &str, kind: &str) -> Result<EpicControlOutcome, String> {
        if epic_id.trim().is_empty() || !matches!(kind, "pause" | "restart") {
            return Err("Epic control request is invalid".into());
        }
        self.validate_epic(epic_id)?;
        let action_id = self.materialize_action(epic_id, kind)?;
        self.reconcile(&action_id)?;
        self.outcome(&action_id)
    }
    pub(crate) fn query(&self, epic_id: &str) -> Result<EpicControlQuery, String> {
        self.validate_epic(epic_id)?;
        let pause = self.latest_outcome(epic_id, "pause")?;
        let restart = self.latest_outcome(epic_id, "restart")?;
        let pause_candidates = self.candidate_count(epic_id, "pause")?;
        let restart_candidates = self.candidate_count(epic_id, "restart")?;
        let pause_busy = pause
            .as_ref()
            .is_some_and(|action| matches!(action.status.as_str(), "pending" | "partial"));
        let restart_busy = restart
            .as_ref()
            .is_some_and(|action| matches!(action.status.as_str(), "pending" | "partial"));
        Ok(EpicControlQuery {
            epic_id: epic_id.into(),
            pause: EpicControlRead {
                availability: if pause_busy {
                    "busy"
                } else if pause_candidates > 0 {
                    "available"
                } else {
                    "unavailable"
                }
                .into(),
                reason: if pause_busy {
                    "A durable Pause request is still reconciling."
                } else {
                    if pause_candidates > 0 {
                        "Pause dispatch is available."
                    } else {
                        "No working orchestration conversation is eligible for Pause."
                    }
                }
                .into(),
                current: pause,
            },
            restart: EpicControlRead {
                availability: if pause_busy || restart_busy {
                    "busy"
                } else {
                    if restart_candidates > 0 {
                        "available"
                    } else {
                        "unavailable"
                    }
                }
                .into(),
                reason: if pause_busy {
                    "Restart waits for the current Pause dispatch."
                } else if restart_busy {
                    "A durable Restart request is still reconciling."
                } else {
                    if restart_candidates > 0 {
                        "Restart dispatch is available."
                    } else {
                        "No interrupted orchestration conversation is eligible for Restart."
                    }
                }
                .into(),
                current: restart,
            },
        })
    }

    pub(crate) fn on_agent_notification(
        &self,
        notification: &AgentSessionNotification,
    ) -> Result<(), String> {
        let AgentSessionNotification::InvocationTerminal { invocation, .. } = notification else {
            return Ok(());
        };
        let ids = self.action_ids_for_invocation(invocation.id.as_str())?;
        for id in ids {
            self.reconcile(&id)?;
        }
        Ok(())
    }

    pub(crate) fn reconcile_startup(&self) -> Result<(), String> {
        let ids = self.action_ids("status IN ('pending','partial')")?;
        for id in ids {
            self.reconcile(&id)?;
        }
        Ok(())
    }

    fn materialize_action(&self, epic_id: &str, kind: &str) -> Result<String, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        if let Some(id) = transaction.query_row("SELECT id FROM epic_control_actions WHERE epic_id=?1 AND kind=?2 AND status IN ('pending','partial') ORDER BY requested_at DESC LIMIT 1", params![epic_id, kind], |r| r.get(0)).optional().map_err(|e| e.to_string())? {
            transaction.commit().map_err(|e| e.to_string())?;
            return Ok(id);
        }
        let id = format!("epic-control-{}", Uuid::new_v4());
        transaction.execute("INSERT INTO epic_control_actions(id,epic_id,kind,status,requested_at) VALUES(?1,?2,?3,'pending',?4)", params![id, epic_id, kind, Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
        let targets = if kind == "pause" {
            active_targets(&transaction, epic_id)?
        } else {
            restart_targets(&transaction, epic_id)?
        };
        for (session_id, source_id, interruption) in targets {
            transaction.execute("INSERT INTO epic_control_targets(action_id,session_id,source_invocation_id,interruption_status) VALUES(?1,?2,?3,?4)", params![id, session_id, source_id, interruption]).map_err(|e| e.to_string())?;
        }
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    fn reconcile(&self, action_id: &str) -> Result<(), String> {
        let (kind, targets) = self.targets(action_id)?;
        for target in targets {
            if kind == "pause" {
                self.reconcile_pause_target(action_id, &target)?;
            } else {
                self.reconcile_restart_target(action_id, &target)?;
            }
        }
        self.refresh_action_status(action_id)
    }

    fn reconcile_pause_target(&self, action_id: &str, target: &Target) -> Result<(), String> {
        match target.interruption_status.as_str() {
            "awaiting_cancel" => {
                let source = AgentInvocationId::new(target.source_invocation_id.clone())
                    .map_err(|e| e.to_string())?;
                let history = self
                    .sessions
                    .load_session(
                        &AgentSessionId::new(target.session_id.clone())
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                let Some(invocation) = history
                    .invocations
                    .into_iter()
                    .find(|x| x.invocation.id == source)
                    .map(|x| x.invocation)
                else {
                    return self.attention(
                        action_id,
                        target,
                        "source_missing",
                        "The selected active invocation is unavailable.",
                    );
                };
                if invocation.status.is_active() {
                    if let Err(error) = self.sessions.cancel_invocation(
                        CancelAgentInvocationCommand {
                            invocation_id: source,
                        },
                    ) {
                        // A port failure is durable target evidence, not a reason to abandon the
                        // other independently-correlated targets in this action.
                        self.record_failure(
                            action_id,
                            target,
                            "cancellation_failed",
                            &error.to_string(),
                        )?;
                        return Ok(());
                    }
                    self.mark_cancel_requested(action_id, target)?;
                    return Ok(());
                }
                self.observe_interruption(action_id, target, invocation.status)?;
                if matches!(
                    invocation.status,
                    AgentInvocationStatus::Canceled | AgentInvocationStatus::Interrupted
                ) {
                    self.dispatch_message(action_id, target, "pause work")?;
                }
            }
            "canceled" | "interrupted" => self.dispatch_message(action_id, target, "pause work")?,
            "failed" | "completed" => self.attention(
                action_id,
                target,
                "not_pause_interrupted",
                "The original work ended before pause interruption was observed.",
            )?,
            _ => {}
        }
        Ok(())
    }

    fn reconcile_restart_target(&self, action_id: &str, target: &Target) -> Result<(), String> {
        if target.interruption_status == "failed"
            || target.interruption_status == "interrupted"
            || target.interruption_status == "canceled"
        {
            self.dispatch_message(action_id, target, "continue work")?;
        }
        Ok(())
    }

    fn dispatch_message(&self, action_id: &str, target: &Target, text: &str) -> Result<(), String> {
        if target.launch_accepted_at.is_some() {
            return Ok(());
        }
        let session = AgentSessionId::new(target.session_id.clone()).map_err(|e| e.to_string())?;
        let invocation = target
            .message_invocation_id
            .clone()
            .unwrap_or_else(|| format!("{}-{}", action_id, target.source_invocation_id));
        let invocation_id =
            AgentInvocationId::new(invocation.clone()).map_err(|e| e.to_string())?;
        match self
            .sessions
            .application_invocation_launch_evidence(&invocation_id, &session)
            .map_err(|e| e.to_string())?
        {
            ApplicationInvocationLaunchEvidence::LaunchAccepted => {
                self.mark_message(action_id, target, &invocation, true)
            }
            ApplicationInvocationLaunchEvidence::PersistedNotAccepted => {
                self.mark_message(action_id, target, &invocation, false)?;
                self.record_failure(action_id, target, "launch_not_accepted", "The application observed persisted message intent but no invocation launch acceptance.")
            }
            ApplicationInvocationLaunchEvidence::NeverPersisted => match self
                .sessions
                .send_idempotent_application_message_with_launch_observation(
                    SendIdempotentApplicationAgentSessionMessageCommand {
                        invocation_id,
                        message: SendAgentSessionMessageCommand {
                            session_id: Some(session),
                            submitted_text: text.into(),
                            title: None,
                            working_directory: None,
                            requested_options: None,
                        },
                    },
                    None,
                ) {
                Ok(result) => {
                    self.mark_message(action_id, target, &invocation, result.launch_accepted)
                }
                Err(error) => {
                    self.record_failure(action_id, target, "dispatch_failed", &error.to_string())
                }
            },
        }
    }

    fn targets(&self, action_id: &str) -> Result<(String, Vec<Target>), String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let kind = c
            .query_row(
                "SELECT kind FROM epic_control_actions WHERE id=?1",
                [action_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut statement=c.prepare("SELECT session_id,source_invocation_id,interruption_status,message_invocation_id,launch_accepted_at FROM epic_control_targets WHERE action_id=?1").map_err(|e|e.to_string())?;
        let values = statement
            .query_map([action_id], |r| {
                Ok(Target {
                    session_id: r.get(0)?,
                    source_invocation_id: r.get(1)?,
                    interruption_status: r.get(2)?,
                    message_invocation_id: r.get(3)?,
                    launch_accepted_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok((kind, values))
    }
    fn action_ids(&self, where_clause: &str) -> Result<Vec<String>, String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let mut statement = c
            .prepare(&format!(
                "SELECT id FROM epic_control_actions WHERE {where_clause}"
            ))
            .map_err(|e| e.to_string())?;
        let ids = statement
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(ids)
    }
    fn action_ids_for_invocation(&self, source: &str) -> Result<Vec<String>, String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let mut statement = c
            .prepare(
                "SELECT DISTINCT action_id FROM epic_control_targets WHERE source_invocation_id=?1 OR message_invocation_id=?1",
            )
            .map_err(|e| e.to_string())?;
        let ids = statement
            .query_map([source], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(ids)
    }
    fn mark_cancel_requested(&self, a: &str, t: &Target) -> Result<(), String> {
        self.update_target(
            a,
            t,
            "cancel_requested_at=COALESCE(cancel_requested_at,?3)",
            params![a, t.source_invocation_id, Utc::now().to_rfc3339()],
        )
    }
    fn observe_interruption(
        &self,
        a: &str,
        t: &Target,
        status: AgentInvocationStatus,
    ) -> Result<(), String> {
        let value = match status {
            AgentInvocationStatus::Canceled => "canceled",
            AgentInvocationStatus::Interrupted => "interrupted",
            AgentInvocationStatus::Failed => "failed",
            AgentInvocationStatus::Completed => "completed",
            _ => "awaiting_cancel",
        };
        self.update_target(
            a,
            t,
            "interruption_status=?3,interruption_observed_at=COALESCE(interruption_observed_at,?4)",
            params![a, t.source_invocation_id, value, Utc::now().to_rfc3339()],
        )
    }
    fn mark_message(&self, a: &str, t: &Target, id: &str, accepted: bool) -> Result<(), String> {
        self.update_target(a,t,"message_invocation_id=COALESCE(message_invocation_id,?3),message_persisted_at=COALESCE(message_persisted_at,?4),launch_accepted_at=CASE WHEN ?5 THEN COALESCE(launch_accepted_at,?4) ELSE launch_accepted_at END,failure_category=CASE WHEN ?5 THEN NULL ELSE failure_category END,failure_detail=CASE WHEN ?5 THEN NULL ELSE failure_detail END",params![a,t.source_invocation_id,id,Utc::now().to_rfc3339(),accepted])
    }
    fn record_failure(
        &self,
        a: &str,
        t: &Target,
        category: &str,
        detail: &str,
    ) -> Result<(), String> {
        self.update_target(
            a,
            t,
            "failure_category=?3,failure_detail=?4",
            params![a, t.source_invocation_id, category, detail],
        )
    }
    fn attention(&self, a: &str, t: &Target, category: &str, detail: &str) -> Result<(), String> {
        self.record_failure(a, t, category, detail)
    }
    fn update_target<P: rusqlite::Params>(
        &self,
        _a: &str,
        _t: &Target,
        set: &str,
        params: P,
    ) -> Result<(), String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        c.execute(&format!("UPDATE epic_control_targets SET {set} WHERE action_id=?1 AND source_invocation_id=?2"),params).map_err(|e|e.to_string())?;
        Ok(())
    }
    fn refresh_action_status(&self, a: &str) -> Result<(), String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        c.execute("UPDATE epic_control_actions SET status=CASE WHEN EXISTS(SELECT 1 FROM epic_control_targets WHERE action_id=?1 AND failure_category IN ('source_missing','not_pause_interrupted','launch_not_accepted')) THEN 'attention' WHEN NOT EXISTS(SELECT 1 FROM epic_control_targets WHERE action_id=?1 AND launch_accepted_at IS NULL) THEN 'completed' WHEN EXISTS(SELECT 1 FROM epic_control_targets WHERE action_id=?1 AND (message_persisted_at IS NOT NULL OR failure_category IS NOT NULL)) THEN 'partial' ELSE 'pending' END, completed_at=CASE WHEN NOT EXISTS(SELECT 1 FROM epic_control_targets WHERE action_id=?1 AND launch_accepted_at IS NULL) THEN COALESCE(completed_at,?2) ELSE completed_at END WHERE id=?1",params![a,Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
        Ok(())
    }
    fn outcome(&self, a: &str) -> Result<EpicControlOutcome, String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let (kind, status): (String, String) = c
            .query_row(
                "SELECT kind,status FROM epic_control_actions WHERE id=?1",
                [a],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let (total,launched):(i64,i64)=c.query_row("SELECT count(*),COALESCE(sum(launch_accepted_at IS NOT NULL),0) FROM epic_control_targets WHERE action_id=?1",[a],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        Ok(EpicControlOutcome {
            action_id: a.into(),
            kind,
            status,
            target_count: total as usize,
            launched_count: launched as usize,
        })
    }
    fn latest_outcome(
        &self,
        epic_id: &str,
        kind: &str,
    ) -> Result<Option<EpicControlOutcome>, String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let id = c.query_row("SELECT id FROM epic_control_actions WHERE epic_id=?1 AND kind=?2 ORDER BY requested_at DESC LIMIT 1", params![epic_id, kind], |r| r.get::<_, String>(0)).optional().map_err(|e| e.to_string())?;
        drop(c);
        id.map(|id| self.outcome(&id)).transpose()
    }
    fn validate_epic(&self, epic_id: &str) -> Result<(), String> {
        let c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let found = c
            .query_row(
                "SELECT 1 FROM epic_initiations WHERE epic_id=?1",
                [epic_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some();
        if found {
            Ok(())
        } else {
            Err("Epic controls are unavailable because the Epic is not durably initiated.".into())
        }
    }
    fn candidate_count(&self, epic_id: &str, kind: &str) -> Result<usize, String> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| "Epic control database lock is poisoned")?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let count = if kind == "pause" {
            active_targets(&tx, epic_id)?.len()
        } else {
            restart_targets(&tx, epic_id)?.len()
        };
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }
}
struct Target {
    session_id: String,
    source_invocation_id: String,
    interruption_status: String,
    message_invocation_id: Option<String>,
    launch_accepted_at: Option<String>,
}

fn active_targets(
    transaction: &rusqlite::Transaction<'_>,
    epic_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let mut statement=transaction.prepare("SELECT DISTINCT session_id, invocation_id FROM (SELECT b.bootstrap_session_id session_id,i.id invocation_id FROM epic_bootstrap_transitions b JOIN agent_session_invocations i ON i.session_id=b.bootstrap_session_id WHERE b.epic_id=?1 AND i.status IN ('pending','running') UNION SELECT b.runner_session_id,i.id FROM epic_bootstrap_transitions b JOIN agent_session_invocations i ON i.session_id=b.runner_session_id WHERE b.epic_id=?1 AND i.status IN ('pending','running') UNION SELECT s.epic_runner_session_id,i.id FROM sprint_runner_transitions s JOIN agent_session_invocations i ON i.session_id=s.epic_runner_session_id WHERE s.epic_id=?1 AND i.status IN ('pending','running') UNION SELECT s.sprint_runner_session_id,i.id FROM sprint_runner_transitions s JOIN agent_session_invocations i ON i.session_id=s.sprint_runner_session_id WHERE s.epic_id=?1 AND i.status IN ('pending','running') UNION SELECT p.planner_session_id,i.id FROM work_slice_planner_transitions p JOIN sprint_runner_transitions s ON s.sprint_id=p.sprint_id JOIN agent_session_invocations i ON i.session_id=p.planner_session_id WHERE s.epic_id=?1 AND i.status IN ('pending','running')) candidate WHERE NOT EXISTS(SELECT 1 FROM epic_control_targets control WHERE control.message_invocation_id=candidate.invocation_id)").map_err(|e|e.to_string())?;
    let targets = statement
        .query_map([epic_id], |r| {
            Ok((r.get(0)?, r.get(1)?, "awaiting_cancel".to_string()))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(targets)
}
fn restart_targets(
    transaction: &rusqlite::Transaction<'_>,
    epic_id: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let mut statement=transaction.prepare("WITH membership(session_id) AS (SELECT bootstrap_session_id FROM epic_bootstrap_transitions WHERE epic_id=?1 UNION SELECT runner_session_id FROM epic_bootstrap_transitions WHERE epic_id=?1 UNION SELECT epic_runner_session_id FROM sprint_runner_transitions WHERE epic_id=?1 UNION SELECT sprint_runner_session_id FROM sprint_runner_transitions WHERE epic_id=?1 UNION SELECT p.planner_session_id FROM work_slice_planner_transitions p JOIN sprint_runner_transitions s ON s.sprint_id=p.sprint_id WHERE s.epic_id=?1), eligible(session_id,source_invocation_id,interruption_status) AS (SELECT target.session_id,target.source_invocation_id,target.interruption_status FROM epic_control_targets target JOIN epic_control_actions action ON action.id=target.action_id JOIN agent_session_invocations pause_message ON pause_message.id=target.message_invocation_id WHERE action.epic_id=?1 AND action.kind='pause' AND target.interruption_status IN ('canceled','interrupted') AND target.launch_accepted_at IS NOT NULL AND pause_message.status NOT IN ('pending','running') AND NOT EXISTS(SELECT 1 FROM agent_session_invocations later WHERE later.session_id=target.session_id AND later.created_at > pause_message.created_at AND later.id NOT IN (SELECT message_invocation_id FROM epic_control_targets WHERE message_invocation_id IS NOT NULL)) UNION SELECT m.session_id,i.id,i.status FROM membership m JOIN agent_session_invocations i ON i.session_id=m.session_id WHERE i.status IN ('failed','interrupted') AND NOT EXISTS(SELECT 1 FROM agent_session_invocations later WHERE later.session_id=i.session_id AND later.created_at > i.created_at AND later.id NOT IN (SELECT message_invocation_id FROM epic_control_targets WHERE message_invocation_id IS NOT NULL))) SELECT DISTINCT session_id,source_invocation_id,interruption_status FROM eligible WHERE NOT EXISTS(SELECT 1 FROM epic_control_targets prior JOIN epic_control_actions prior_action ON prior_action.id=prior.action_id WHERE prior_action.kind='restart' AND prior.source_invocation_id=eligible.source_invocation_id)").map_err(|e|e.to_string())?;
    let targets = statement
        .query_map([epic_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, String>(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{
            application::{
                AgentSessionNotifier,
                CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
                SendAgentSessionMessageCommand,
                SendIdempotentApplicationAgentSessionMessageCommand,
                SystemAgentSessionProviders, AgentSessionApplication,
            },
            domain::{AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeOptions, AgentSessionId},
            ports::{
                AgentRuntime, AgentRuntimeUpdateSink, RuntimeInvocationMode,
                RuntimeInvocationOutcome, RuntimeInvocationPreflight, RuntimeInvocationRequest,
                RuntimePortError, RuntimePortErrorKind, RuntimeUpdate,
            },
            repository::SqliteAgentSessionRepository,
        },
        storage::{initialize_active_database, open_active_database},
    };
    use rusqlite::Connection;
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Weak,
        },
    };

    #[derive(Default)]
    struct RecordedRuntime {
        requests: Mutex<Vec<RuntimeInvocationRequest>>,
        sinks: Mutex<HashMap<AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>>>,
        cancellations: Mutex<Vec<AgentInvocationId>>,
        fail_cancellations: AtomicUsize,
        fail_launches: AtomicUsize,
    }

    impl RecordedRuntime {
        fn requests_with_text(&self, text: &str) -> Vec<RuntimeInvocationRequest> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.submitted_text == text)
                .cloned()
                .collect()
        }

        fn finish(&self, invocation_id: &str, status: AgentInvocationTerminalStatus) {
            let invocation_id = AgentInvocationId::new(invocation_id).unwrap();
            let sink = self.sinks.lock().unwrap().get(&invocation_id).cloned().unwrap();
            sink.emit_update(
                &invocation_id,
                RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                    status,
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                }),
            )
            .unwrap();
        }
    }

    impl AgentRuntime for RecordedRuntime {
        fn preflight_invocation(
            &self,
            _mode: RuntimeInvocationMode,
            requested_options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight { effective_options: requested_options.clone() })
        }

        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.requests.lock().unwrap().push(request.clone());
            if self.fail_launches.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| count.checked_sub(1)).is_ok() {
                return Err(RuntimePortError::new(RuntimePortErrorKind::LaunchFailed, "induced launch failure"));
            }
            self.sinks.lock().unwrap().insert(request.invocation_id, sink);
            Ok(())
        }

        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _external_context_id: crate::agent_sessions::domain::ExternalRuntimeContextId,
            sink: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.start_invocation(request, sink)
        }

        fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
            self.cancellations.lock().unwrap().push(invocation_id.clone());
            if self.fail_cancellations.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| count.checked_sub(1)).is_ok() {
                return Err(RuntimePortError::new(RuntimePortErrorKind::CancellationFailed, "induced cancellation failure"));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct ControlNotifier { service: Mutex<Option<Weak<EpicPauseRestartService>>> }

    impl ControlNotifier {
        fn set(&self, service: &Arc<EpicPauseRestartService>) {
            *self.service.lock().unwrap() = Some(Arc::downgrade(service));
        }
    }

    impl AgentSessionNotifier for ControlNotifier {
        fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
            let service = self.service.lock().unwrap().as_ref().and_then(Weak::upgrade);
            if let Some(service) = service {
                service.on_agent_notification(&notification)?;
            }
            Ok(())
        }
    }

    struct Fixture {
        directory: tempfile::TempDir,
        database_path: std::path::PathBuf,
        runtime: Arc<RecordedRuntime>,
        sessions: Arc<AgentSessionApplication>,
        service: Arc<EpicPauseRestartService>,
        source_ids: Vec<String>,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let database_path = directory.path().join("active.sqlite");
            open_active_database(&database_path).unwrap();
            let runtime = Arc::new(RecordedRuntime::default());
            let notifier = Arc::new(ControlNotifier::default());
            let sessions = Arc::new(AgentSessionApplication::new(
                Arc::new(SqliteAgentSessionRepository::open(&database_path).unwrap()),
                runtime.clone(), notifier.clone(), Arc::new(SystemAgentSessionProviders),
                Arc::new(SystemAgentSessionProviders), Some("test-runtime".into()),
            ));
            let names = ["bootstrap", "runner", "epic-runner", "sprint-runner", "planner", "unrelated"];
            for name in names {
                sessions.create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: AgentSessionId::new(format!("{name}-session")).unwrap(),
                    session: CreateAgentSessionCommand { title: Some(name.into()), working_directory: None, requested_options: AgentRuntimeOptions::default() },
                }).unwrap();
            }
            let mut source_ids = Vec::new();
            for name in names {
                let source = format!("{name}-source");
                sessions.send_idempotent_application_message_with_launch_observation(
                    SendIdempotentApplicationAgentSessionMessageCommand {
                        invocation_id: AgentInvocationId::new(source.clone()).unwrap(),
                        message: SendAgentSessionMessageCommand { session_id: Some(AgentSessionId::new(format!("{name}-session")).unwrap()), submitted_text: format!("{name} original work"), title: None, working_directory: None, requested_options: None },
                    }, None,
                ).unwrap();
                source_ids.push(source);
            }
            let seed = Connection::open(&database_path).unwrap();
            seed.pragma_update(None, "foreign_keys", false).unwrap();
            seed.execute_batch("CREATE TABLE IF NOT EXISTS sprint_runner_transitions (sprint_id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, epic_runner_session_id TEXT NOT NULL, sprint_runner_session_id TEXT NOT NULL); CREATE TABLE IF NOT EXISTS work_slice_planner_transitions (sprint_id TEXT PRIMARY KEY, planner_session_id TEXT NOT NULL);").unwrap();
            seed.execute_batch("INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('initiation','command','result','event','provenance','draft','revision','snapshot','epic','t'); INSERT INTO epic_bootstrap_transitions (initiation_id,epic_id,proposal_revision_id,material_snapshot_hash,proposal_json,preparation_id,prepared_root,approved_plan_path,manifest_path,overview_path,runner_brief_path,bootstrap_session_id,bootstrap_invocation_id,runner_session_id,runner_invocation_id,created_at,updated_at) VALUES ('initiation','epic','revision','hash','{}','preparation','root','plan','manifest','overview','brief','bootstrap-session','bootstrap-source','runner-session','runner-source','t','t'); INSERT INTO sprint_runner_transitions VALUES ('sprint','epic','epic-runner-session','sprint-runner-session'); INSERT INTO work_slice_planner_transitions VALUES ('sprint','planner-session');").unwrap();
            seed.pragma_update(None, "foreign_keys", true).unwrap();
            drop(seed);
            let service = EpicPauseRestartService::open(&database_path, sessions.clone()).unwrap();
            notifier.set(&service);
            Self { directory, database_path, runtime, sessions, service, source_ids }
        }

        fn target_rows(&self, action_id: &str) -> Vec<(String, String, Option<String>, Option<String>)> {
            let connection = Connection::open(&self.database_path).unwrap();
            let mut statement = connection.prepare("SELECT session_id, source_invocation_id, message_invocation_id, failure_category FROM epic_control_targets WHERE action_id=?1 ORDER BY session_id").unwrap();
            statement.query_map([action_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap().collect::<Result<_, _>>().unwrap()
        }
    }

    #[test]
    fn control_schema_is_idempotent_for_fresh_and_reopened_databases() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(EPIC_PAUSE_RESTART_SCHEMA)
            .expect("fresh schema");
        connection
            .execute_batch(EPIC_PAUSE_RESTART_SCHEMA)
            .expect("reopen schema");
        for table in ["epic_control_actions", "epic_control_targets"] {
            assert!(connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |_| Ok(())
                )
                .is_ok());
        }
    }

    #[test]
    fn control_target_keeps_one_effect_per_action_and_source_interruption() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch("CREATE TABLE agent_sessions (id TEXT PRIMARY KEY);")
            .unwrap();
        connection
            .execute_batch(EPIC_PAUSE_RESTART_SCHEMA)
            .expect("schema");
        connection
            .execute("INSERT INTO agent_sessions VALUES ('session-1')", [])
            .unwrap();
        connection.execute("INSERT INTO epic_control_actions VALUES ('action-1','epic-1','pause','pending','now',NULL)", []).unwrap();
        connection.execute("INSERT INTO epic_control_targets(action_id,session_id,source_invocation_id,interruption_status) VALUES ('action-1','session-1','source-1','awaiting_cancel')", []).unwrap();
        assert!(connection.execute("INSERT INTO epic_control_targets(action_id,session_id,source_invocation_id,interruption_status) VALUES ('action-1','session-1','source-1','awaiting_cancel')", []).is_err());
    }

    #[test]
    fn active_v12_initialization_adds_the_control_schema_and_records_v13() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA).unwrap();
        connection.execute_batch(crate::orchestration::repository::ORCHESTRATION_SCHEMA).unwrap();
        connection.execute_batch(crate::orchestration::repository::ORCHESTRATION_INITIATION_SCHEMA).unwrap();
        connection.pragma_update(None, "user_version", 12).unwrap();
        initialize_active_database(&connection).unwrap();
        assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), 13);
        assert!(connection.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name='epic_control_actions'", [], |_| Ok(())).is_ok());
        initialize_active_database(&connection).unwrap();
    }

    #[test]
    fn pause_and_restart_use_only_durable_epic_membership_and_application_lifecycle_notifications() {
        let fixture = Fixture::new();
        let pause = fixture.service.request("epic", "pause").unwrap();
        assert_eq!(pause.target_count, 5);
        assert_eq!(fixture.runtime.cancellations.lock().unwrap().len(), 5);
        assert!(!fixture.runtime.cancellations.lock().unwrap().iter().any(|id| id.as_str() == "unrelated-source"));
        assert!(fixture.runtime.requests_with_text("pause work").is_empty());

        for source in fixture.source_ids.iter().take(5) {
            fixture.runtime.finish(source, AgentInvocationTerminalStatus::Canceled);
        }
        assert_eq!(fixture.runtime.requests_with_text("pause work").len(), 5);
        let pause_rows = fixture.target_rows(&pause.action_id);
        assert_eq!(pause_rows.len(), 5);
        assert!(pause_rows.iter().all(|(_, _, message, failure)| message.is_some() && failure.is_none()));

        for (_, _, message, _) in &pause_rows {
            fixture.runtime.finish(message.as_deref().unwrap(), AgentInvocationTerminalStatus::Completed);
        }
        // A later ordinary invocation is not a reattachment point for this interrupted work.
        fixture.sessions.send_message(SendAgentSessionMessageCommand {
            session_id: Some(AgentSessionId::new("bootstrap-session").unwrap()),
            submitted_text: "independent later work".into(),
            title: None,
            working_directory: None,
            requested_options: None,
        }).unwrap();
        let restart = fixture.service.request("epic", "restart").unwrap();
        assert_eq!(restart.target_count, 4);
        assert_eq!(fixture.runtime.requests_with_text("continue work").len(), 4);
        let repeated = fixture.service.request("epic", "restart").unwrap();
        assert_ne!(repeated.action_id, restart.action_id);
        assert_eq!(repeated.target_count, 0);
        assert_eq!(fixture.runtime.requests_with_text("continue work").len(), 4);

        let reopened = EpicPauseRestartService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        reopened.reconcile_startup().unwrap();
        assert_eq!(fixture.runtime.requests_with_text("continue work").len(), 4);
    }

    #[test]
    fn one_cancellation_failure_settles_partial_without_abandoning_other_correlated_targets() {
        let fixture = Fixture::new();
        fixture.runtime.fail_cancellations.store(1, Ordering::SeqCst);
        let pause = fixture.service.request("epic", "pause").unwrap();
        assert_eq!(pause.status, "partial");
        assert_eq!(fixture.runtime.cancellations.lock().unwrap().len(), 5);
        let rows = fixture.target_rows(&pause.action_id);
        assert_eq!(rows.iter().filter(|(_, _, _, failure)| failure.as_deref() == Some("cancellation_failed")).count(), 1);
    }

    #[test]
    fn persisted_but_not_launch_accepted_pause_effect_requires_attention_after_reconciliation() {
        let fixture = Fixture::new();
        let pause = fixture.service.request("epic", "pause").unwrap();
        fixture.runtime.fail_launches.store(1, Ordering::SeqCst);
        fixture.runtime.finish("bootstrap-source", AgentInvocationTerminalStatus::Canceled);
        let initial = fixture.service.query("epic").unwrap().pause.current.unwrap();
        assert_eq!(initial.status, "partial");

        let reopened = EpicPauseRestartService::open(&fixture.database_path, fixture.sessions.clone()).unwrap();
        reopened.reconcile_startup().unwrap();
        let settled = reopened.query("epic").unwrap().pause.current.unwrap();
        assert_eq!(settled.action_id, pause.action_id);
        assert_eq!(settled.status, "attention");
        assert_eq!(fixture.target_rows(&pause.action_id).iter().filter(|(_, _, _, failure)| failure.as_deref() == Some("launch_not_accepted")).count(), 1);
    }
}
