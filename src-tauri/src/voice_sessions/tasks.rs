use super::{domain::*, VoiceSessionApplication};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssignmentInput {
    pub task: String,
    pub title: Option<String>,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub profile_id: Option<String>,
    pub working_directory: Option<String>,
    pub parent_assignment_id: Option<String>,
}

pub(super) fn assign(
    c: &mut Conversation,
    caller: &str,
    caller_invocation: Option<&str>,
    recipient: &str,
    input: &AssignmentInput,
) -> Result<Delegation, String> {
    if input.task.trim().is_empty() {
        return Err("task is required".into());
    }
    let parent = if let Some(id) = &input.parent_assignment_id {
        Some(
            c.delegations
                .iter()
                .find(|d| d.id == *id && d.session_id == caller)
                .ok_or("The parent assignment does not belong to the caller")?
                .clone(),
        )
    } else if c.delegations.iter().any(|d| d.session_id == caller) {
        c.delegations
            .iter()
            .rev()
            .find(|d| {
                d.session_id == caller
                    && caller_invocation.is_none_or(|id| d.invocation_id.as_deref() == Some(id))
            })
            .cloned()
    } else {
        None
    };
    let task_id = input
        .task_id
        .clone()
        .or_else(|| {
            parent
                .as_ref()
                .map(|d| d.task_id.clone())
                .filter(|id| !id.is_empty())
        })
        .or_else(|| {
            c.delegations
                .iter()
                .rev()
                .find(|d| d.session_id == recipient)
                .map(|d| d.task_id.clone())
                .filter(|id| !id.is_empty())
        });
    let now = chrono::Utc::now().to_rfc3339();
    let (task_id, revision) = if let Some(id) = task_id {
        let task = c
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("Unknown taskId; read_context lists current tasks")?;
        if parent.is_none() {
            task.revision += 1;
            task.request = input.task.clone();
            task.updated_at = now.clone();
        }
        let revision = parent
            .as_ref()
            .filter(|p| p.task_id == task.id)
            .map(|p| p.revision)
            .unwrap_or(task.revision);
        (task.id.clone(), revision)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        c.tasks.push(VoiceTask {
            id: id.clone(),
            title: input
                .title
                .clone()
                .unwrap_or_else(|| input.task.chars().take(80).collect()),
            revision: 1,
            request: input.task.clone(),
            updated_at: now.clone(),
        });
        (id, 1)
    };
    let assignment = Delegation {
        destination_node_id: String::new(),
        request_key: String::new(),
        id: uuid::Uuid::new_v4().to_string(),
        task_id,
        revision,
        parent_assignment_id: parent.map(|d| d.id),
        delivery: None,
        outcome: None,
        runtime_status: None,
        session_id: recipient.into(),
        from_session_id: caller.into(),
        return_session_id: caller.into(),
        task: input.task.clone(),
        invocation_id: None,
        created_at: now,
    };
    c.delegations.push(assignment.clone());
    Ok(assignment)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutcomeInput {
    assignment_id: String,
    revision: u64,
    summary: String,
    #[serde(default)]
    material_refs: Vec<String>,
    #[serde(default)]
    unresolved: Vec<String>,
}
impl VoiceSessionApplication {
    pub(crate) fn report_outcome(
        self: &Arc<Self>,
        caller: &str,
        invocation: &str,
        args: Value,
    ) -> Result<Value, String> {
        let input: OutcomeInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        if input.summary.trim().is_empty() {
            return Err("summary is required".into());
        }
        self.require_member(caller)?;
        let (_, source) = self.workflows.otp_source(caller, invocation)?;
        self.update(|s| {
            let c = s.conversation.as_mut().ok_or("No conversation")?;
            if let Some(d) = c.delegations.iter_mut().find(|d| {
                d.id == input.assignment_id
                    && d.session_id.is_empty()
                    && d.destination_node_id == source.node_id
            }) {
                d.session_id = caller.into();
                d.invocation_id = Some(invocation.into());
            }
            Ok(())
        })?;
        let c = self.require_member(caller)?;
        let assignment = c
            .delegations
            .iter()
            .find(|d| d.id == input.assignment_id && d.session_id == caller)
            .ok_or("Assignment does not belong to this worker")?;
        if input.revision != assignment.revision {
            return Err("Report the request revision supplied with the assignment".into());
        }
        if assignment
            .invocation_id
            .as_deref()
            .is_some_and(|id| id != invocation)
        {
            return Err("Outcome belongs to another invocation".into());
        }
        let outcome = json!({"assignmentId":assignment.id,"taskId":assignment.task_id,"revision":input.revision,
            "summary":input.summary,"materialRefs":input.material_refs,"unresolved":input.unresolved,"invocationId":invocation,"origin":"agent_report"});
        self.update(|s| {
            let c = s.conversation.as_mut().ok_or("No conversation")?;
            let a = c
                .delegations
                .iter_mut()
                .find(|d| d.id == input.assignment_id)
                .ok_or("Assignment missing")?;
            a.invocation_id = Some(invocation.into());
            a.outcome = Some(outcome.clone());
            Ok(())
        })?;
        let mut result = outcome;
        result["requesterSessionId"] = assignment.return_session_id.clone().into();
        self.record_activity(caller, "assignment_outcome", result.clone(), &[])?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(text: &str, title: &str) -> AssignmentInput {
        serde_json::from_value(json!({"task":text,"title":title})).unwrap()
    }
    #[test]
    fn two_tasks_and_a_child_keep_their_own_request_versions() {
        let mut c = Conversation {
            main_session_id: "router".into(),
            ..Default::default()
        };
        let a = assign(
            &mut c,
            "router",
            None,
            "audit",
            &input("Investigate", "Audit"),
        )
        .unwrap();
        let b = assign(&mut c, "router", None, "picnic", &input("Plan", "Picnic")).unwrap();
        let update = assign(
            &mut c,
            "router",
            None,
            "picnic",
            &input("Make it vegan", "Picnic"),
        )
        .unwrap();
        assert_eq!(update.task_id, b.task_id);
        assert_eq!(update.revision, 2);
        assert_eq!(
            c.tasks.iter().find(|t| t.id == a.task_id).unwrap().revision,
            1
        );
        let child = assign(
            &mut c,
            "picnic",
            None,
            "research",
            &input("Check venues", "Venue research"),
        )
        .unwrap();
        assert_eq!(child.task_id, b.task_id);
        assert_eq!(child.revision, 2);
        assert_eq!(
            child.parent_assignment_id.as_deref(),
            Some(update.id.as_str())
        );
    }
    #[test]
    fn child_work_uses_the_parent_invocation_revision_before_an_amendment_arrives() {
        let mut c = Conversation {
            main_session_id: "router".into(),
            ..Default::default()
        };
        let original =
            assign(&mut c, "router", None, "worker", &input("Review", "Review")).unwrap();
        c.delegations[0].invocation_id = Some("original-turn".into());
        let amendment = assign(
            &mut c,
            "router",
            None,
            "worker",
            &input("Narrow review", "Review"),
        )
        .unwrap();
        let child = assign(
            &mut c,
            "worker",
            Some("original-turn"),
            "research",
            &input("Inspect", "Research"),
        )
        .unwrap();
        assert_eq!(amendment.revision, 2);
        assert_eq!(child.revision, original.revision);
        assert_eq!(
            child.parent_assignment_id.as_deref(),
            Some(original.id.as_str())
        );
        assert_eq!(c.tasks[0].revision, 2);
    }
}
