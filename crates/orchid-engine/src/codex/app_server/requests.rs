//! Server request projection and response validation; native IDs never become product IDs.
use super::{connection::unavailable, Invocation};
use crate::contracts::ports::{RuntimePortError, RuntimePortErrorKind};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

pub(super) struct PendingRequest {
    native_id: Value,
    choices: Vec<Value>,
    questions: Option<Value>,
}

pub(super) fn receive(invocation: &Invocation, message: Value) -> Result<(), RuntimePortError> {
    let method = message["method"].as_str().unwrap_or("");
    let params = &message["params"];
    if method == "currentTime/read" {
        return invocation.connection.write(
            json!({"id":message["id"],"result":{"currentTimeAt":chrono::Utc::now().timestamp()}}),
        );
    }
    let mut choices = Vec::new();
    let mut questions = None;
    let kind = match method {
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            let decisions = params["availableDecisions"]
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![json!("accept"), json!("decline"), json!("cancel")]);
            for decision in decisions {
                let mut choice = super::approval_choices::project(decision);
                if method == "item/fileChange/requestApproval" {
                    // Command-specific explanations do not apply to file-change requests.
                    choice.as_object_mut().unwrap().remove("description");
                }
                choices.push(choice);
            }
            "approval"
        }
        "item/permissions/requestApproval" => {
            choices = vec![
                json!({"label":"Allow for this turn","response":{"permissions":params["permissions"],"scope":"turn"}}),
                json!({"label":"Decline","response":{"permissions":{},"scope":"turn"}}),
            ];
            "approval"
        }
        "item/tool/requestUserInput" => {
            questions = Some(params["questions"].clone());
            "questions"
        }
        "mcpServer/elicitation/request" if params["mode"] == "url" => {
            choices = vec![
                json!({"label":"Completed","response":{"action":"accept"}}),
                json!({"label":"Decline","response":{"action":"decline"}}),
                json!({"label":"Cancel","response":{"action":"cancel"}}),
            ];
            "external_action"
        }
        _ => "unsupported",
    };
    let request_id = uuid::Uuid::new_v4().to_string();
    let default_title = match kind {
        "approval" => "Approval requested",
        "questions" => "The agent needs your input",
        "external_action" => "Complete the requested action",
        _ => method,
    };
    let request = json!({"id":request_id,"kind":kind,"title":params["reason"].as_str().or(params["message"].as_str()).unwrap_or(default_title),"command":params["command"],"cwd":params["cwd"],"url":params["url"],"permissions":params["permissions"],"grantRoot":params["grantRoot"],"choices":choices,"questions":questions,"supported":kind != "unsupported"});
    if kind == "unsupported" {
        invocation
            .event(json!({"kind":"runtime_request_unsupported","request":request,"method":method}));
        return invocation.connection.write(json!({"id":message["id"],"error":{"code":-32601,"message":format!("Orchestrator does not support {method}")}}));
    }
    invocation
        .requests
        .lock()
        .map_err(|_| unavailable("Request lock poisoned"))?
        .insert(
            request_id,
            PendingRequest {
                native_id: message["id"].clone(),
                choices: choices.iter().map(|c| c["response"].clone()).collect(),
                questions,
            },
        );
    invocation.sink.emit_update(
        &invocation.connection.invocation_id,
        crate::contracts::ports::RuntimeUpdate::Event(crate::contracts::ports::RuntimeEventDraft {
            source: crate::contracts::domain::AgentRuntimeEventSource::Runtime,
            raw_payload: json!({"kind":"runtime_request_opened","request":request}),
            normalized: None,
        }),
    )?;
    Ok(())
}

pub(super) fn respond(
    invocation: &Invocation,
    request_id: &str,
    response: Value,
) -> Result<(), RuntimePortError> {
    if invocation.finished.load(Ordering::Acquire) {
        return Err(rejected("The request expired when its turn finished"));
    }
    let pending = {
        let mut requests = invocation
            .requests
            .lock()
            .map_err(|_| unavailable("Request lock poisoned"))?;
        let request = requests
            .get(request_id)
            .ok_or_else(|| rejected("Request is no longer pending"))?;
        validate(request, &response)?;
        requests
            .remove(request_id)
            .expect("request checked under lock")
    };
    // Writing a response has no separate provider acknowledgement. Never retry after an uncertain write.
    invocation
        .connection
        .write(json!({"id":pending.native_id,"result":response}))
}

fn validate(request: &PendingRequest, response: &Value) -> Result<(), RuntimePortError> {
    if request.choices.contains(response) {
        return Ok(());
    }
    if let Some(questions) = request.questions.as_ref().and_then(Value::as_array) {
        let answers = response["answers"]
            .as_object()
            .ok_or_else(|| rejected("Answers must be supplied by question ID"))?;
        if answers.len() != questions.len() {
            return Err(rejected("Answer every question exactly once"));
        }
        for question in questions {
            let id = question["id"]
                .as_str()
                .ok_or_else(|| rejected("Malformed runtime question"))?;
            let values = answers
                .get(id)
                .and_then(|v| v["answers"].as_array())
                .ok_or_else(|| rejected("Missing question answer"))?;
            if values.is_empty() || values.iter().any(|v| !v.is_string()) {
                return Err(rejected("Question answers must contain text"));
            }
            if question["isOther"] != true {
                if let Some(options) = question["options"].as_array() {
                    if values
                        .iter()
                        .any(|v| !options.iter().any(|option| option["label"] == *v))
                    {
                        return Err(rejected("Answer is outside the offered choices"));
                    }
                }
            }
        }
        return Ok(());
    }
    Err(rejected("Response is outside the offered choices"))
}

fn rejected(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_responses_cannot_expand_the_offered_permission() {
        let request = PendingRequest {
            native_id: json!(1),
            choices: vec![json!({"decision":"accept"}), json!({"decision":"decline"})],
            questions: None,
        };
        assert!(validate(&request, &json!({"decision":"decline"})).is_ok());
        assert!(validate(&request, &json!({"decision":"acceptForSession"})).is_err());
        assert!(validate(
            &request,
            &json!({"decision":"accept","extraPermission":"all"})
        )
        .is_err());
    }

    #[test]
    fn questions_require_each_id_and_respect_offered_or_free_text_answers() {
        let request = PendingRequest {
            native_id: json!(1),
            choices: vec![],
            questions: Some(json!([
                {"id":"choice","options":[{"label":"One"}],"isOther":false},
                {"id":"text","isOther":true,"isSecret":true}
            ])),
        };
        assert!(validate(
            &request,
            &json!({"answers":{"choice":{"answers":["One"]},"text":{"answers":["private answer"]}}})
        )
        .is_ok());
        assert!(validate(&request,&json!({"answers":{"choice":{"answers":["Unlisted"]},"text":{"answers":["private answer"]}}})).is_err());
        assert!(validate(&request, &json!({"answers":{"choice":{"answers":["One"]}}})).is_err());
        assert!(validate(&request,&json!({"answers":{"choice":{"answers":["One"]},"wrongId":{"answers":["private answer"]}}})).is_err());
    }
}
