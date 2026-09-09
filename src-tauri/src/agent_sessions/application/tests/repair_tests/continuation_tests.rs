use super::*;
const CONTINUATION_TOOL: &str = "trigger_workflow_continuation";
use crate::workflows::{compiled_plan::FileAssociation, file_inputs::resolve_node_files};

fn continuation_instance(fixture: &Fixture) -> RecipeInstance {
    let baseline = fixture.instance(None);
    let mut draft = baseline.recipe.clone();
    let mut third = draft.nodes[1].clone();
    third.node_id = "c".into();
    third.name = "Clarification".into();
    draft.nodes.push(third);
    for node in &mut draft.nodes {
        node.node_profile.allowed_capabilities.mcp_tools = [(
            "workflow".into(),
            [CONTINUATION_TOOL.into()].into_iter().collect(),
        )]
        .into_iter()
        .collect();
    }
    let connection = &mut draft.connections[0];
    connection.trigger = otp_output(CONTINUATION_TOOL, "continuation");
    connection.prompt_inputs = vec![
        WorkflowConnectionPromptInput::OutputField {
            field: "outputFiles".into(),
        },
        WorkflowConnectionPromptInput::OutputField {
            field: "sourceNode".into(),
        },
        WorkflowConnectionPromptInput::NodeFiles {
            node_id: "a".into(),
            association: FileAssociation::Either,
        },
    ];
    let mut parallel = connection.clone();
    parallel.connection_id = "a-to-c".into();
    parallel.destination_node_id = "c".into();
    let mut other_node = connection.clone();
    other_node.connection_id = "b-to-a".into();
    other_node.source_node_id = "b".into();
    other_node.destination_node_id = "a".into();
    draft.connections.extend([parallel, other_node]);
    draft = fixture.authoring.save_draft(draft).unwrap().draft;
    fixture
        .authoring
        .activate_revision(&draft.recipe_id, draft.revision)
        .unwrap();
    fixture
        .execution
        .create_instance(
            &draft.recipe_id,
            draft.revision,
            "Continuation".into(),
            fixture.target(),
        )
        .unwrap()
}

fn start(fixture: &Fixture, instance: &RecipeInstance) -> RuntimeInvocationRequest {
    fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Discuss".into())
        .unwrap();
    fixture.launches().last().unwrap().clone()
}

fn proxy_url(launch: &RuntimeInvocationRequest) -> String {
    serde_json::from_str(
        launch
            .launch_extension
            .as_ref()
            .unwrap()
            .additional_args
            .iter()
            .find_map(|arg| arg.strip_prefix("mcp_servers.session_capability_1.url="))
            .unwrap(),
    )
    .unwrap()
}

async fn call(url: &str, arguments: serde_json::Value) -> serde_json::Value {
    reqwest::Client::new()
        .post(url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":CONTINUATION_TOOL,"arguments":arguments}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

fn record(
    fixture: &Fixture,
    launch: &RuntimeInvocationRequest,
    changes: serde_json::Value,
    seconds: i64,
) {
    let history = fixture.sessions.load_session(&launch.session_id).unwrap();
    let invocation = &history.invocations[0];
    let sequence = invocation
        .events
        .last()
        .map_or(1, |event| event.sequence + 1);
    fixture
        .repository
        .append_event(AgentRuntimeEvent {
            id: AgentRuntimeEventId::new(uuid::Uuid::new_v4().to_string()).unwrap(),
            invocation_id: invocation.invocation.id.clone(),
            sequence,
            source: AgentRuntimeEventSource::Stdout,
            raw_payload: json!({"type":"item.completed"}),
            normalized: Some(NormalizedRuntimeEvent {
                kind: NormalizedRuntimeEventKind::ToolActivity,
                text: None,
                external_context_id: None,
                usage: None,
                tool_activity: None,
                details: Some(json!({"fileChanges":changes})),
            }),
            recorded_at: chrono::Utc::now() + chrono::Duration::seconds(seconds),
        })
        .unwrap();
}

#[tokio::test]
async fn continuation_mcp_routes_only_this_node_and_call_without_closing_the_source() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, true);
    let instance = continuation_instance(&fixture);
    let launch = start(&fixture, &instance);
    let url = proxy_url(&launch);
    let listed: serde_json::Value = reqwest::Client::new()
        .post(&url)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tool = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == CONTINUATION_TOOL)
        .unwrap();
    assert_eq!(tool["_meta"]["otp"]["package"], "workflow");
    assert_eq!(
        tool["_meta"]["otp"]["tool"]["outputs"][0]["schema"]["properties"]["outputFiles"]["type"],
        "array"
    );
    for arguments in [
        json!({"_workflowInvocation":{"sourceInvocationId":"foreign"}}),
        json!({"outputFiles":[3]}),
        json!({"sourceNode":"b"}),
    ] {
        let rejected = call(&url, arguments).await;
        assert_eq!(rejected["result"]["isError"], true, "{rejected}");
    }
    assert_eq!(fixture.launches().len(), 1);
    let response = call(&url, json!({"outputFiles":["claimed.md"]})).await;
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(fixture.launches().len(), 3);
    let source_history = fixture.sessions.load_session(&launch.session_id).unwrap();
    assert_eq!(source_history.invocations.len(), 1);
    assert_eq!(
        source_history.invocations[0].invocation.status,
        AgentInvocationStatus::Running
    );
    for receiver in &fixture.launches()[1..] {
        let history = fixture.sessions.load_session(&receiver.session_id).unwrap();
        let text = &history.invocations[0].invocation.submitted_text;
        assert!(text.contains("claimed.md"), "{text}");
        assert!(text.contains("\"id\": \"a\""), "{text}");
        assert!(text.contains("\"files\": []"), "{text}");
    }
    let history = fixture
        .repository
        .file_history_at_scope(
            &ReferenceIdentity::new("workflow", "instance", &instance.id).unwrap(),
        )
        .unwrap();
    assert!(
        history.is_empty(),
        "Published output files do not attribute edits"
    );
    // Node c has no connection for this call, even though a and b have matching tools.
    let no_connections = call(&proxy_url(&fixture.launches()[2]), json!({})).await;
    assert_eq!(
        no_connections["result"]["isError"], false,
        "{no_connections}"
    );
    assert!(no_connections["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("0 prompt delivery"));
    assert_eq!(fixture.launches().len(), 3);
}

#[test]
fn node_file_history_survives_later_editors_archival_and_reopen_and_stays_in_its_instance() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, true);
    let instance = continuation_instance(&fixture);
    let author = start(&fixture, &instance);
    std::fs::write(fixture.folder.path().join("plan.md"), "current").unwrap();
    record(
        &fixture,
        &author,
        json!([
            {"path":"plan.md","operation":"create"},
            {"path":"missing.md","operation":"create"},
            {"path":"../outside.md","operation":"create"},
            {"path":"existing.md","operation":"edit"}
        ]),
        0,
    );
    fixture
        .execution
        .invoke_mcp(
            "workflow",
            CONTINUATION_TOOL,
            author.session_id.as_str(),
            author.invocation_id.as_str(),
            json!({}),
        )
        .unwrap();
    let reviewer = fixture.launches()[1].clone();
    let clarification = fixture.launches()[2].clone();
    record(
        &fixture,
        &reviewer,
        json!([{"path":"plan.md","operation":"edit"}]),
        1,
    );
    record(
        &fixture,
        &clarification,
        json!([{"path":"plan.md","operation":"edit"}]),
        2,
    );
    fixture
        .repository
        .set_session_availability(
            &reviewer.session_id,
            AgentSessionAvailability::Archived,
            chrono::Utc::now(),
        )
        .unwrap();
    let reopened =
        SqliteAgentSessionRepository::open(fixture.folder.path().join("repair.sqlite")).unwrap();
    let selected: serde_json::Value = serde_json::from_str(
        &resolve_node_files(&instance, "b", FileAssociation::Edited, &reopened).unwrap(),
    )
    .unwrap();
    assert_eq!(selected["nodeId"], "b");
    assert_eq!(selected["files"].as_array().unwrap().len(), 1);
    assert_eq!(selected["files"][0]["path"], "plan.md");
    assert_eq!(selected["files"][0]["exists"], true);
    assert_eq!(selected["files"][0]["created"]["nodeId"], "a");
    assert_eq!(selected["files"][0]["lastEdited"]["nodeId"], "c");
    let created: serde_json::Value = serde_json::from_str(
        &resolve_node_files(&instance, "a", FileAssociation::Created, &reopened).unwrap(),
    )
    .unwrap();
    assert_eq!(created["files"].as_array().unwrap().len(), 2);
    assert_eq!(created["files"][0]["path"], "missing.md");
    assert_eq!(created["files"][0]["exists"], false);
    let edited: serde_json::Value = serde_json::from_str(
        &resolve_node_files(&instance, "a", FileAssociation::Edited, &reopened).unwrap(),
    )
    .unwrap();
    assert_eq!(edited["files"].as_array().unwrap().len(), 1);
    assert_eq!(edited["files"][0]["created"], serde_json::Value::Null);
    let other = fixture
        .execution
        .create_instance(
            &instance.recipe.recipe_id,
            instance.recipe.revision,
            "Other".into(),
            fixture.target(),
        )
        .unwrap();
    let other_files: serde_json::Value = serde_json::from_str(
        &resolve_node_files(&other, "a", FileAssociation::Either, &reopened).unwrap(),
    )
    .unwrap();
    assert_eq!(other_files["files"], json!([]));
}
