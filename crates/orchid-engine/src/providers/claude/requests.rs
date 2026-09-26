//! Claude's permission prompts (`can_use_tool`) as Orchid runtime requests, and the answers back
//! as `control_response` messages. `AskUserQuestion` arrives as a permission prompt whose
//! approval carries the answers.
use super::{connection::unavailable, runtime::Invocation};
use crate::contracts::{
    ports::{RuntimePortError, RuntimePortErrorKind},
    RuntimeControlRecord, RuntimeInteractionResponse, RuntimeQuestion, RuntimeQuestionOption,
    RuntimeRequest, RuntimeRequestChoice, RuntimeRequestKind,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) struct PendingRequest {
    native_id: String,
    choices: BTreeMap<String, Value>,
    /// Orchid's questions and the tool input they came from.
    questions: Option<(Vec<RuntimeQuestion>, Value)>,
}

pub(super) fn receive(invocation: &Invocation, message: &Value) -> Result<(), RuntimePortError> {
    let native_id = message["request_id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let request = &message["request"];
    let subtype = request["subtype"].as_str().unwrap_or_default();
    let id = uuid::Uuid::new_v4().to_string();
    let (projected, pending) = match subtype {
        "can_use_tool" if request["tool_name"] == "AskUserQuestion" => questions(&id, request),
        "can_use_tool" => approval(&id, request),
        _ => {
            invocation.control(RuntimeControlRecord::RequestUnsupported {
                request: RuntimeRequest {
                    id,
                    title: subtype.into(),
                    ..Default::default()
                },
                method: subtype.into(),
            });
            return invocation
                .connection
                .write(json!({"type":"control_response","response":{
                "subtype":"error","request_id":native_id,
                "error":format!("Orchid does not support {subtype}")}}));
        }
    };
    invocation
        .requests
        .lock()
        .map_err(|_| unavailable("Request lock poisoned"))?
        .insert(
            id,
            PendingRequest {
                native_id,
                ..pending
            },
        );
    invocation.control(RuntimeControlRecord::RequestOpened { request: projected });
    Ok(())
}

fn approval(id: &str, request: &Value) -> (RuntimeRequest, PendingRequest) {
    let input = &request["input"];
    let tool = request["display_name"]
        .as_str()
        .or(request["tool_name"].as_str())
        .unwrap_or("a tool");
    let mut choices = vec![choice("allow", "Allow once", None)];
    let mut native = BTreeMap::from([(
        "allow".to_owned(),
        json!({"behavior":"allow","updatedInput":input}),
    )]);
    let rules: Vec<&Value> = request["permission_suggestions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|suggestion| suggestion["type"] == "addRules")
        .collect();
    if !rules.is_empty() {
        let scope = rules
            .iter()
            .flat_map(|suggestion| suggestion["rules"].as_array().into_iter().flatten())
            .map(|rule| match rule["ruleContent"].as_str() {
                Some(content) => {
                    format!("{}({content})", rule["toolName"].as_str().unwrap_or(tool))
                }
                None => rule["toolName"].as_str().unwrap_or(tool).to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        choices.push(choice("allow_always", "Always allow", Some(scope)));
        native.insert(
            "allow_always".into(),
            json!({"behavior":"allow","updatedInput":input,"updatedPermissions":rules}),
        );
    }
    choices.push(choice("decline", "Decline", None));
    native.insert(
        "decline".into(),
        json!({"behavior":"deny","message":"The user declined this action."}),
    );
    let projected = RuntimeRequest {
        id: id.into(),
        kind: RuntimeRequestKind::Approval,
        title: request["description"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Allow {tool}")),
        command: input["command"].as_str().map(str::to_owned),
        grant_root: request["blocked_path"].as_str().map(str::to_owned),
        permissions: input["command"].is_null().then(|| input.clone()),
        choices,
        supported: true,
        ..Default::default()
    };
    (
        projected,
        PendingRequest {
            native_id: String::new(),
            choices: native,
            questions: None,
        },
    )
}

fn questions(id: &str, request: &Value) -> (RuntimeRequest, PendingRequest) {
    let input = request["input"].clone();
    let questions: Vec<RuntimeQuestion> = input["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, question)| RuntimeQuestion {
            id: format!("q{}", index + 1),
            header: question["header"].as_str().map(str::to_owned),
            question: question["question"].as_str().unwrap_or_default().to_owned(),
            options: question["options"].as_array().map(|options| {
                options
                    .iter()
                    .map(|option| RuntimeQuestionOption {
                        label: option["label"].as_str().unwrap_or_default().to_owned(),
                        description: option["description"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                    })
                    .collect()
            }),
            multi_select: question["multiSelect"] == true,
            // Claude always accepts a typed answer alongside the options.
            is_other: true,
            is_secret: false,
        })
        .collect();
    let projected = RuntimeRequest {
        id: id.into(),
        kind: RuntimeRequestKind::Questions,
        title: "Claude needs your input".into(),
        questions: Some(questions.clone()),
        supported: true,
        ..Default::default()
    };
    (
        projected,
        PendingRequest {
            native_id: String::new(),
            choices: BTreeMap::new(),
            questions: Some((questions, input)),
        },
    )
}

fn choice(id: &str, label: &str, scope: Option<String>) -> RuntimeRequestChoice {
    RuntimeRequestChoice {
        id: id.into(),
        label: label.into(),
        description: None,
        scope,
    }
}

pub(super) fn respond(
    invocation: &Invocation,
    request_id: &str,
    response: RuntimeInteractionResponse,
) -> Result<(), RuntimePortError> {
    if invocation.is_finished() {
        return Err(rejected("The request expired when its turn finished"));
    }
    let (native_id, native_response) = {
        let mut requests = invocation
            .requests
            .lock()
            .map_err(|_| unavailable("Request lock poisoned"))?;
        let request = requests
            .get(request_id)
            .ok_or_else(|| rejected("Request is no longer pending"))?;
        let encoded = encode(request, &response)?;
        let request = requests
            .remove(request_id)
            .expect("request checked under lock");
        (request.native_id, encoded)
    };
    // Claude sends no acknowledgement. Never retry after an uncertain write.
    invocation
        .connection
        .write(json!({"type":"control_response","response":{
        "subtype":"success","request_id":native_id,"response":native_response}}))
}

fn encode(
    request: &PendingRequest,
    response: &RuntimeInteractionResponse,
) -> Result<Value, RuntimePortError> {
    match (response, &request.questions) {
        (RuntimeInteractionResponse::Choose { choice_id }, _) => request
            .choices
            .get(choice_id)
            .cloned()
            .ok_or_else(|| rejected("Response is outside the offered choices")),
        (RuntimeInteractionResponse::Answer { answers }, Some((questions, input))) => {
            if answers.len() != questions.len() {
                return Err(rejected("Answer every question exactly once"));
            }
            let mut native = serde_json::Map::new();
            for question in questions {
                let values = answers
                    .get(&question.id)
                    .ok_or_else(|| rejected("Missing question answer"))?;
                question.validate_answer(values).map_err(rejected)?;
                native.insert(question.question.clone(), values.join(", ").into());
            }
            let mut updated = input.clone();
            updated["answers"] = Value::Object(native);
            Ok(json!({"behavior":"allow","updatedInput":updated}))
        }
        (RuntimeInteractionResponse::Answer { .. }, None) => {
            Err(rejected("Response is outside the offered choices"))
        }
    }
}

fn rejected(message: &str) -> RuntimePortError {
    RuntimePortError::new(RuntimePortErrorKind::UnsupportedOptions, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_request(name: &str) -> Value {
        super::super::tests::fixture(name)
            .into_iter()
            .find(|message| message["type"] == "control_request")
            .expect("fixture has a permission prompt")["request"]
            .clone()
    }

    #[test]
    fn approvals_offer_once_always_and_decline_without_widening() {
        let (projected, pending) = approval("id", &fixture_request("approval"));
        let ids: Vec<_> = projected
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect();
        assert_eq!(ids, ["allow", "allow_always", "decline"]);
        assert_eq!(projected.command.as_deref(), Some("touch orchid-probe.txt"));
        assert_eq!(
            projected.choices[1].scope.as_deref(),
            Some("Bash(touch orchid-probe.txt)")
        );
        let always = encode(
            &pending,
            &RuntimeInteractionResponse::Choose {
                choice_id: "allow_always".into(),
            },
        )
        .unwrap();
        assert_eq!(always["updatedPermissions"][0]["type"], "addRules");
        assert_eq!(always["updatedPermissions"].as_array().unwrap().len(), 1);
        let declined = encode(
            &pending,
            &RuntimeInteractionResponse::Choose {
                choice_id: "decline".into(),
            },
        )
        .unwrap();
        assert_eq!(declined["behavior"], "deny");
        assert!(encode(
            &pending,
            &RuntimeInteractionResponse::Choose {
                choice_id: "other".into()
            }
        )
        .is_err());
    }

    #[test]
    fn question_answers_return_under_their_question_text() {
        let (projected, pending) = questions("id", &fixture_request("question"));
        let question = &projected.questions.as_ref().unwrap()[0];
        assert_eq!(
            (question.id.as_str(), question.header.as_deref()),
            ("q1", Some("Color"))
        );
        assert!(question.is_other && !question.multi_select);
        let answer = |value: &str| RuntimeInteractionResponse::Answer {
            answers: BTreeMap::from([("q1".into(), vec![value.into()])]),
        };
        let native = encode(&pending, &answer("Blue")).unwrap();
        assert_eq!(
            native["updatedInput"]["answers"]["Which color do you prefer?"],
            "Blue"
        );
        assert_eq!(native["updatedInput"]["questions"][0]["header"], "Color");
        assert!(encode(&pending, &answer("Green, typed")).is_ok());
        let two = RuntimeInteractionResponse::Answer {
            answers: BTreeMap::from([("q1".into(), vec!["Red".into(), "Blue".into()])]),
        };
        assert!(encode(&pending, &two).is_err());
    }
}
