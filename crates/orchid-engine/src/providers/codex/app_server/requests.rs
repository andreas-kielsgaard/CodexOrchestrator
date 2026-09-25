//! Server request projection and response validation; native IDs never become product IDs.
use super::{connection::unavailable, Invocation};
use crate::contracts::{ports::{RuntimePortError, RuntimePortErrorKind}, RuntimeInteractionResponse};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

pub(super) struct PendingRequest {
    native_id: Value,
    choices: BTreeMap<String, Value>,
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
    let mut native_choices = BTreeMap::new();
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
                let id = format!("choice-{}", choices.len() + 1);
                native_choices.insert(id.clone(), choice["response"].clone());
                choice.as_object_mut().unwrap().remove("response");
                choice["id"] = id.into();
                choices.push(choice);
            }
            "approval"
        }
        "item/permissions/requestApproval" => {
            native_choices.insert("allow_turn".into(), json!({"permissions":params["permissions"],"scope":"turn"}));
            native_choices.insert("decline".into(), json!({"permissions":{},"scope":"turn"}));
            choices = vec![
                json!({"id":"allow_turn","label":"Allow for this turn"}),
                json!({"id":"decline","label":"Decline"}),
            ];
            "approval"
        }
        "item/tool/requestUserInput" => {
            questions = Some(params["questions"].clone());
            "questions"
        }
        "mcpServer/elicitation/request" if params["mode"] == "url" => {
            native_choices.insert("completed".into(), json!({"action":"accept"}));
            native_choices.insert("decline".into(), json!({"action":"decline"}));
            native_choices.insert("cancel".into(), json!({"action":"cancel"}));
            choices = vec![
                json!({"id":"completed","label":"Completed"}),
                json!({"id":"decline","label":"Decline"}),
                json!({"id":"cancel","label":"Cancel"}),
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
                choices: native_choices,
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
    response: RuntimeInteractionResponse,
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
        let native_response = encode(request, &response)?;
        requests
            .remove(request_id)
            .map(|request| (request, native_response))
            .expect("request checked under lock")
    };
    // Writing a response has no separate provider acknowledgement. Never retry after an uncertain write.
    invocation
        .connection
        .write(json!({"id":pending.0.native_id,"result":pending.1}))
}

fn encode(request: &PendingRequest, response: &RuntimeInteractionResponse) -> Result<Value, RuntimePortError> {
    if let RuntimeInteractionResponse::Choose { choice_id } = response {
        return request.choices.get(choice_id).cloned().ok_or_else(|| rejected("Response is outside the offered choices"));
    }
    if let Some(questions) = request.questions.as_ref().and_then(Value::as_array) {
        let RuntimeInteractionResponse::Answer { answers } = response else {
            return Err(rejected("Answers must be supplied by question ID"));
        };
        if answers.len() != questions.len() {
            return Err(rejected("Answer every question exactly once"));
        }
        for question in questions {
            let id = question["id"]
                .as_str()
                .ok_or_else(|| rejected("Malformed runtime question"))?;
            let values = answers
                .get(id)
                .ok_or_else(|| rejected("Missing question answer"))?;
            if values.is_empty() || values.iter().any(|v| v.is_empty()) {
                return Err(rejected("Question answers must contain text"));
            }
            if question["isOther"] != true {
                if let Some(options) = question["options"].as_array() {
                    if values
                        .iter()
                        .any(|v| !options.iter().any(|option| option["label"] == v.as_str()))
                    {
                        return Err(rejected("Answer is outside the offered choices"));
                    }
                }
            }
        }
        return Ok(json!({"answers":answers.iter().map(|(id, answers)| (id.clone(), json!({"answers":answers}))).collect::<serde_json::Map<_,_>>() }));
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
            choices: BTreeMap::from([
                ("accept".into(), json!({"decision":"accept"})),
                ("decline".into(), json!({"decision":"decline"})),
            ]),
            questions: None,
        };
        assert_eq!(encode(&request, &RuntimeInteractionResponse::Choose { choice_id: "decline".into() }).unwrap(), json!({"decision":"decline"}));
        assert!(encode(&request, &RuntimeInteractionResponse::Choose { choice_id: "accept_for_session".into() }).is_err());
    }

    #[test]
    fn questions_require_each_id_and_respect_offered_or_free_text_answers() {
        let request = PendingRequest {
            native_id: json!(1),
            choices: BTreeMap::new(),
            questions: Some(json!([
                {"id":"choice","options":[{"label":"One"}],"isOther":false},
                {"id":"text","isOther":true,"isSecret":true}
            ])),
        };
        let answers = |choice: &str, second_id: &str| RuntimeInteractionResponse::Answer {
            answers: BTreeMap::from([
                ("choice".into(), vec![choice.into()]),
                (second_id.into(), vec!["private answer".into()]),
            ]),
        };
        assert!(encode(&request, &answers("One", "text")).is_ok());
        assert!(encode(&request, &answers("Unlisted", "text")).is_err());
        assert!(encode(&request, &RuntimeInteractionResponse::Answer { answers: BTreeMap::from([("choice".into(), vec!["One".into()])]) }).is_err());
        assert!(encode(&request, &answers("One", "wrongId")).is_err());
    }
}
