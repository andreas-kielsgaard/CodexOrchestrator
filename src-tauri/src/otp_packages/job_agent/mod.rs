use crate::otp_api::*;
use serde_json::Value;

pub(crate) struct JobAgentPackage;

const TOOLS: &[(&str, &str)] = &[
    ("get_source_catalog", "source_discovery"),
    ("get_source_detail", "source_discovery"),
    ("get_source_discovery_context", "source_discovery"),
    ("submit_source_suggestions", "source_discovery"),
    ("get_source_setup_state", "source_registry"),
    ("create_source", "source_registry"),
    ("update_source", "source_registry"),
    ("archive_source", "source_registry"),
    ("restore_source", "source_registry"),
    ("capture_source_calibration", "source_calibration"),
    ("list_source_artifacts", "source_calibration"),
    ("get_recipe_evidence", "source_calibration"),
    ("create_recipe_candidate", "recipe_authoring"),
    ("get_recipe_candidate", "recipe_authoring"),
    ("update_recipe_candidate", "recipe_authoring"),
    ("reject_recipe_candidate", "recipe_authoring"),
    ("approve_recipe_candidate", "recipe_authoring"),
    ("adopt_recipe_candidate", "recipe_authoring"),
    ("prepare_execution_projection", "source_execution"),
    ("refresh_execution_projection", "source_execution"),
    ("get_source_readiness", "source_execution"),
    ("run_safe_source_test", "source_execution"),
    ("enable_source_when_ready", "source_execution"),
    ("disable_source", "source_execution"),
    ("get_source_session_status", "source_session_status"),
];

const GRANTS: &[(&str, &str, &str)] = &[
    (
        "registryConfigure",
        "Allow source registry configuration",
        "source_registry:configure",
    ),
    (
        "calibrationCalibrate",
        "Allow source calibration",
        "source_calibration:calibrate",
    ),
    (
        "recipeAuthor",
        "Allow recipe authoring",
        "recipe_authoring:author_recipe",
    ),
    (
        "recipeApprove",
        "Allow recipe approval",
        "recipe_authoring:approve_recipe",
    ),
    (
        "recipeAdopt",
        "Allow recipe adoption",
        "recipe_authoring:adopt_recipe",
    ),
    (
        "executionConfigure",
        "Allow execution projection configuration",
        "source_execution:configure",
    ),
    (
        "executionTest",
        "Allow safe source tests",
        "source_execution:test",
    ),
    (
        "executionEnable",
        "Allow source enablement",
        "source_execution:enable",
    ),
];

pub(crate) fn grants_from_configuration(value: &Value) -> Result<Vec<(String, String)>, String> {
    let object = value
        .as_object()
        .ok_or("Job Agent configuration must be an object")?;
    let mut result = vec![];
    for (key, _, grant) in GRANTS {
        match object.get(*key).and_then(Value::as_str).unwrap_or("deny") {
            "deny" => {}
            "allow" => {
                let (capability, transition) = grant.split_once(':').unwrap();
                result.push((capability.into(), transition.into()));
            }
            _ => return Err(format!("Invalid Job Agent grant value for {key}")),
        }
    }
    if let Some(key) = object
        .keys()
        .find(|key| !GRANTS.iter().any(|(known, _, _)| known == key))
    {
        return Err(format!("Unknown Job Agent configuration field {key}"));
    }
    Ok(result)
}

fn fields() -> Vec<ConfigurationField> {
    GRANTS
        .iter()
        .map(|(key, label, _)| ConfigurationField {
            key: (*key).into(),
            label: (*label).into(),
            choices: vec!["deny".into(), "allow".into()],
            default_value: "deny".into(),
            when: None,
        })
        .collect()
}

impl OtpPackage for JobAgentPackage {
    fn descriptor(&self) -> PackageDescriptor {
        PackageDescriptor {
            id: "job_agent".into(),
            contract_version: 1,
            requested_handles: vec![],
            tools: vec![],
            agent_mcp_servers: vec![AgentMcpServerDescriptor {
                server_name: "job_agent".into(),
                name: "Job Agent".into(),
                description: "Guarded local source setup operations.".into(),
                tools: TOOLS
                    .iter()
                    .map(|(id, capability)| AgentMcpToolDescriptor {
                        id: (*id).into(),
                        name: id.replace('_', " "),
                        description: format!("Job Agent tool {id}."),
                        capability: (*capability).into(),
                    })
                    .collect(),
                configuration: fields(),
            }],
        }
    }

    fn validate_configuration(&self, tool: &str, _: &Value) -> Result<(), String> {
        Err(format!("Job Agent has no Workflow tool named {tool}"))
    }

    fn validate_agent_mcp_configuration(
        &self,
        server: &str,
        configuration: &Value,
    ) -> Result<(), String> {
        if server != "job_agent" {
            return Err(format!("Job Agent does not provide MCP server {server}"));
        }
        grants_from_configuration(configuration).map(|_| ())
    }

    fn invoke(
        &self,
        _: &InvocationContext,
        _: ToolInput,
        _: &dyn crate::otp_api::OtpHost,
    ) -> Result<ToolResult, String> {
        Err("Job Agent does not provide a Workflow trigger or action".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grants_are_validated() {
        assert_eq!(
            grants_from_configuration(&json!({"registryConfigure":"allow"})).unwrap(),
            vec![("source_registry".into(), "configure".into())]
        );
        assert!(grants_from_configuration(&json!({"unknown":"allow"})).is_err());
    }
}
