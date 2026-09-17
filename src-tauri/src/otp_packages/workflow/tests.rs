use super::*;
use std::sync::Mutex;
#[derive(Default)]
struct Host {
    emitted: Mutex<Vec<(String, Value)>>,
    sessions: Vec<NodeSession>,
}
impl OtpHost for Host {
    fn node(&self, _: &InvocationContext, id: &str) -> Result<NodeDefinition, String> {
        Ok(NodeDefinition {
            id: id.into(),
            name: format!("Node {id}"),
            initial_prompt: None,
            configuration: json!({}),
        })
    }
    fn connection(&self, _: &InvocationContext) -> Result<Option<ConnectionDefinition>, String> {
        Ok(None)
    }
    fn sessions(&self, _: &InvocationContext, _: &str) -> Result<Vec<NodeSession>, String> {
        Ok(self.sessions.clone())
    }
    fn emit(
        &self,
        _: &InvocationContext,
        output: &str,
        payload: Value,
    ) -> Result<RoutingReceipt, String> {
        self.emitted.lock().unwrap().push((output.into(), payload));
        Ok(RoutingReceipt {
            deliveries: 2,
            stops: 0,
        })
    }
}
fn context(tool: &str) -> InvocationContext {
    InvocationContext {
        instance_id: "instance".into(),
        occurrence_id: "occurrence".into(),
        capability: CapabilityRef {
            package: "workflow".into(),
            tool: tool.into(),
        },
        source: Some(SourceContext {
            node_id: "source".into(),
            node_name: "Source".into(),
            session_id: "sender".into(),
            invocation_id: "turn".into(),
        }),
        connection_id: Some("edge".into()),
        output_node_id: Some("destination".into()),
    }
}
#[test]
fn package_emits_declared_data_without_consuming_the_source_session() {
    let host = Host::default();
    let package = WorkflowPackage;
    let result = package
        .invoke(
            &context("trigger_workflow_continuation"),
            ToolInput::Mcp(json!({"outputFiles":["spec.md"]})),
            &host,
        )
        .unwrap();
    assert!(result.session_requests.is_empty());
    assert!(result.text.contains("2 prompt delivery"));
    assert_eq!(
        host.emitted.lock().unwrap()[0],
        (
            "continuation".into(),
            json!({"outputFiles":["spec.md"],"sourceNode":{"id":"source","name":"Node source"},"data":{}})
        )
    );
    package
        .invoke(
            &context("trigger_workflow_continuation"),
            ToolInput::Mcp(json!({})),
            &host,
        )
        .unwrap();
    assert_eq!(host.emitted.lock().unwrap()[1].1["outputFiles"], json!([]));
    assert!(package
        .invoke(
            &context("trigger_workflow_continuation"),
            ToolInput::Mcp(json!({"sourceNode":"other"})),
            &host
        )
        .is_err());
    package
        .invoke(
            &context("handoff_to_agent"),
            ToolInput::Mcp(json!({"filePaths":["a","b"],"promptText":"Read"})),
            &host,
        )
        .unwrap();
    assert_eq!(
        host.emitted.lock().unwrap()[2].1["filePaths"],
        json!(["a", "b"])
    );
}
#[test]
fn completed_consumer_filters_terminal_facts() {
    let host = Host::default();
    for status in ["failed", "cancelled", "completed"] {
        WorkflowPackage
            .invoke(
                &context("on_invocation_completed"),
                ToolInput::SessionEvent {
                    status: status.into(),
                    output: "done".into(),
                },
                &host,
            )
            .unwrap();
    }
    assert_eq!(
        *host.emitted.lock().unwrap(),
        vec![(
            "completed".into(),
            json!({"output":"done","sourceNode":{"id":"source","name":"Node source"}})
        )]
    );
}
fn session(id: &str, sequence: u64, addressed: Option<u64>) -> NodeSession {
    NodeSession {
        id: id.into(),
        running: false,
        created_sequence: sequence,
        last_addressed_sequence: addressed,
        created_by_event: None,
        created_by_session: None,
    }
}
#[test]
fn prompt_policy_requests_exact_and_fresh_sessions_from_available_population() {
    let host = Host {
        sessions: vec![session("old", 1, Some(9)), session("new", 2, None)],
        ..Host::default()
    };
    let run = |configuration| {
        WorkflowPackage
            .invoke(
                &context("prompt_agent"),
                ToolInput::Action {
                    configuration,
                    inputs: vec![ResolvedInput {
                        reference: "input".into(),
                        value: json!({"files":["spec.md"]}),
                    }],
                },
                &host,
            )
            .unwrap()
            .session_requests
    };
    assert_eq!(
        run(json!({}))[0].target,
        SessionRequestTarget::Exact {
            session_id: "new".into()
        }
    );
    assert_eq!(
        run(json!({"ordering":"last_addressed"}))[0].target,
        SessionRequestTarget::Exact {
            session_id: "old".into()
        }
    );
    assert_eq!(
        run(json!({"mode":"new"}))[0].target,
        SessionRequestTarget::New
    );
    assert_eq!(run(json!({"cardinality":"all"})).len(), 2);
    assert!(run(json!({"running":"running_only","missing":"noop"})).is_empty());
    assert_eq!(
        run(json!({"createdBySession":"unknown"}))[0].target,
        SessionRequestTarget::New
    );
    assert_eq!(
        serde_json::from_str::<Value>(&run(json!({}))[0].prompt[0].text).unwrap(),
        json!({"files":["spec.md"]})
    );
}
#[test]
fn product_imports_define_available_tools() {
    let empty = crate::otp_host::OtpRegistry::import(&[]).unwrap();
    assert!(empty.catalogue().is_empty());
    assert!(empty.tool(&context("prompt_agent").capability).is_err());
    let registry = crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap();
    assert_eq!(
        registry.mcp_tools()["workflow"]
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["handoff_to_agent", "trigger_workflow_continuation"]
    );
    assert!(crate::otp_host::OtpRegistry::import(&["workflow", "workflow"]).is_err());
    assert!(crate::otp_host::OtpRegistry::import(&["missing"]).is_err());
    assert!(registry
        .validate_configuration(
            &context("prompt_agent").capability,
            &json!({"mode":"invalid"})
        )
        .is_err());
}

#[test]
fn catalogue_serialization_supplies_the_designer_fixture() {
    let registry = crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap();
    let value = serde_json::to_value(registry.catalogue()).unwrap();
    if let Ok(path) = std::env::var("OTP_CATALOGUE_FIXTURE_OUTPUT") {
        std::fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/features/workflowAuthoring/otpCatalogue.fixture.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture, value);
}

#[test]
fn stop_selects_one_running_session_with_configured_ordering() {
    let make = |id: &str, running, created, addressed| NodeSession {
        id: id.into(),
        running,
        created_sequence: created,
        last_addressed_sequence: Some(addressed),
        created_by_event: None,
        created_by_session: None,
    };
    let host = Host {
        sessions: vec![
            make("idle", false, 9, 99),
            make("newest", true, 3, 4),
            make("addressed", true, 1, 8),
        ],
        ..Host::default()
    };
    let run = |value| {
        WorkflowPackage
            .invoke(
                &context("stop_session"),
                ToolInput::Action {
                    configuration: value,
                    inputs: vec![],
                },
                &host,
            )
            .unwrap()
    };
    let newest = run(json!({}));
    assert_eq!(newest.stop_requests.len(), 1);
    assert_eq!(newest.stop_requests[0].session_id, "newest");
    assert_eq!(
        run(json!({"ordering":"last_addressed"})).stop_requests[0].session_id,
        "addressed"
    );
    assert!(newest.session_requests.is_empty());
    assert!(host.emitted.lock().unwrap().is_empty());
    assert!(WorkflowPackage
        .validate_configuration("stop_session", &json!({"mode":"new"}))
        .is_err());
}
