use crate::otp_api::*;
use serde_json::{json, Map, Value};

pub(crate) struct JobAgentPackage;

struct CapabilityGroup {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}
const CAPABILITY_GROUPS: &[CapabilityGroup] = &[
    CapabilityGroup {
        id: "source_discovery",
        name: "Source discovery",
        description:
            "Read source context and save reviewable suggestions without changing the registry.",
    },
    CapabilityGroup {
        id: "source_registry",
        name: "Source registry",
        description: "Read and maintain source records while keeping execution disabled.",
    },
    CapabilityGroup {
        id: "source_calibration",
        name: "Source calibration",
        description:
            "Capture bounded public evidence and inspect artifacts used to author recipes.",
    },
    CapabilityGroup {
        id: "recipe_authoring",
        name: "Recipe authoring",
        description: "Create, review, approve, and adopt constrained source recipe candidates.",
    },
    CapabilityGroup {
        id: "source_execution",
        name: "Source execution",
        description: "Prepare, test, enable, disable, and inspect guarded execution projections.",
    },
    CapabilityGroup {
        id: "source_session_status",
        name: "Source session status",
        description: "Read usable-session status without revealing credentials or browser control.",
    },
];

struct Grant {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    capability: &'static str,
    transition: &'static str,
}
const GRANTS: &[Grant] = &[
    Grant {
        id: "registryConfigure",
        label: "Allow source registry configuration",
        description: "Create, update, archive, and restore source records.",
        capability: "source_registry",
        transition: "configure",
    },
    Grant {
        id: "calibrationCalibrate",
        label: "Allow source calibration",
        description: "Capture public calibration evidence for a source.",
        capability: "source_calibration",
        transition: "calibrate",
    },
    Grant {
        id: "recipeAuthor",
        label: "Allow recipe authoring",
        description: "Create, update, and reject pending recipe candidates.",
        capability: "recipe_authoring",
        transition: "author_recipe",
    },
    Grant {
        id: "recipeApprove",
        label: "Allow recipe approval",
        description: "Approve a pending recipe candidate into a validated recipe.",
        capability: "recipe_authoring",
        transition: "approve_recipe",
    },
    Grant {
        id: "recipeAdopt",
        label: "Allow recipe adoption",
        description: "Adopt an approved recipe for a source.",
        capability: "recipe_authoring",
        transition: "adopt_recipe",
    },
    Grant {
        id: "executionConfigure",
        label: "Allow execution projection configuration",
        description: "Prepare, refresh, and disable execution projections.",
        capability: "source_execution",
        transition: "configure",
    },
    Grant {
        id: "executionTest",
        label: "Allow safe source tests",
        description: "Run guarded source tests without normal run outputs.",
        capability: "source_execution",
        transition: "test",
    },
    Grant {
        id: "executionEnable",
        label: "Allow source enablement",
        description: "Enable a source when current readiness checks pass.",
        capability: "source_execution",
        transition: "enable",
    },
];

type Fields = &'static [(&'static str, &'static str)];
struct Endpoint {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    capability: &'static str,
    inputs: Fields,
    required: &'static [&'static str],
    outputs: Fields,
    grants: &'static [&'static str],
}
const ENDPOINTS: &[Endpoint] = &[
    Endpoint { id:"get_source_catalog", name:"Get source catalog", description:"Read compact source registry, health, value, and readiness summaries.", capability:"source_discovery", inputs:&[("statuses","array"),("kinds","array")], required:&[], outputs:&[("sources","array")], grants:&[] },
    Endpoint { id:"get_source_detail", name:"Get source detail", description:"Read one source's lifecycle, candidates, readiness, calibration summaries, and session status.", capability:"source_discovery", inputs:&[("source_id","string")], required:&["source_id"], outputs:&[("source","object"),("readiness","object"),("session","object")], grants:&[] },
    Endpoint { id:"get_source_discovery_context", name:"Get source discovery context", description:"Prepare profile-derived discovery context with existing sources and disqualified domains.", capability:"source_discovery", inputs:&[("focus","string")], required:&[], outputs:&[("context_id","string"),("focus","string"),("context","object")], grants:&[] },
    Endpoint { id:"submit_source_suggestions", name:"Submit source suggestions", description:"Validate and save a review-only source suggestion set.", capability:"source_discovery", inputs:&[("context_id","string"),("suggestions","array")], required:&["context_id","suggestions"], outputs:&[("result_id","string"),("suggestions","array"),("disqualified","array")], grants:&[] },
    Endpoint { id:"get_source_setup_state", name:"Get source setup state", description:"Read complete source setup state, blockers, revision tokens, and allowed next actions.", capability:"source_registry", inputs:&[("source_id","string")], required:&["source_id"], outputs:&[("source","object"),("state","object"),("blockers","array")], grants:&[] },
    Endpoint { id:"create_source", name:"Create source", description:"Create one public source registry record in a disabled state.", capability:"source_registry", inputs:&[("name","string"),("url","string"),("notes","string")], required:&["name","url"], outputs:&[("source","object"),("state","object")], grants:&["registryConfigure"] },
    Endpoint { id:"update_source", name:"Update source", description:"Update source name, public URL, notes, or status using a current revision.", capability:"source_registry", inputs:&[("source_id","string"),("expected_revision","string"),("name","string"),("url","string"),("notes","string"),("status","string")], required:&["source_id","expected_revision"], outputs:&[("source","object"),("state","object")], grants:&["registryConfigure"] },
    Endpoint { id:"archive_source", name:"Archive source", description:"Archive a source and disable any existing execution projection.", capability:"source_registry", inputs:&[("source_id","string"),("expected_revision","string")], required:&["source_id","expected_revision"], outputs:&[("source","object"),("state","object")], grants:&["registryConfigure"] },
    Endpoint { id:"restore_source", name:"Restore source", description:"Restore an archived source without enabling execution.", capability:"source_registry", inputs:&[("source_id","string"),("expected_revision","string"),("status","string")], required:&["source_id","expected_revision"], outputs:&[("source","object"),("state","object")], grants:&["registryConfigure"] },
    Endpoint { id:"capture_source_calibration", name:"Capture source calibration", description:"Capture bounded public calibration evidence for an existing source.", capability:"source_calibration", inputs:&[("source_id","string"),("rendered","boolean"),("capture_detail","boolean"),("max_candidates","number")], required:&["source_id"], outputs:&[("artifact","object"),("source_id","string")], grants:&["calibrationCalibrate"] },
    Endpoint { id:"list_source_artifacts", name:"List source artifacts", description:"List saved calibration artifacts associated with one source.", capability:"source_calibration", inputs:&[("source_id","string")], required:&["source_id"], outputs:&[("source_id","string"),("artifacts","array")], grants:&[] },
    Endpoint { id:"get_recipe_evidence", name:"Get recipe evidence", description:"Read bounded evidence from a listed calibration artifact.", capability:"source_calibration", inputs:&[("source_id","string"),("artifact_id","string"),("view","string"),("cursor","number"),("chunk_size","number")], required:&["source_id","artifact_id"], outputs:&[("evidence","object"),("cursor","number")], grants:&[] },
    Endpoint { id:"create_recipe_candidate", name:"Create recipe candidate", description:"Validate a constrained recipe proposal against saved evidence and save a pending candidate.", capability:"recipe_authoring", inputs:&[("source_id","string"),("artifact_id","string"),("proposal","object")], required:&["source_id","artifact_id","proposal"], outputs:&[("candidate","object")], grants:&["recipeAuthor"] },
    Endpoint { id:"get_recipe_candidate", name:"Get recipe candidate", description:"Read a saved recipe candidate with revision, validation, and quality evidence.", capability:"recipe_authoring", inputs:&[("candidate_id","string")], required:&["candidate_id"], outputs:&[("candidate","object")], grants:&[] },
    Endpoint { id:"update_recipe_candidate", name:"Update recipe candidate", description:"Replace a pending candidate proposal using its current revision.", capability:"recipe_authoring", inputs:&[("source_id","string"),("candidate_id","string"),("artifact_id","string"),("proposal","object"),("expected_revision","string")], required:&["source_id","candidate_id","artifact_id","proposal","expected_revision"], outputs:&[("candidate","object")], grants:&["recipeAuthor"] },
    Endpoint { id:"reject_recipe_candidate", name:"Reject recipe candidate", description:"Reject a pending candidate with an optional reason.", capability:"recipe_authoring", inputs:&[("candidate_id","string"),("expected_revision","string"),("reason","string")], required:&["candidate_id","expected_revision"], outputs:&[("candidate","object")], grants:&["recipeAuthor"] },
    Endpoint { id:"approve_recipe_candidate", name:"Approve recipe candidate", description:"Approve a pending candidate into a validated recipe and preview health.", capability:"recipe_authoring", inputs:&[("source_id","string"),("candidate_id","string"),("expected_revision","string")], required:&["source_id","candidate_id","expected_revision"], outputs:&[("candidate","object"),("recipe","object")], grants:&["recipeApprove"] },
    Endpoint { id:"adopt_recipe_candidate", name:"Adopt recipe candidate", description:"Adopt an approved recipe and optionally prepare a disabled projection.", capability:"recipe_authoring", inputs:&[("source_id","string"),("candidate_id","string"),("expected_revision","string"),("prepare_execution","boolean")], required:&["source_id","candidate_id","expected_revision"], outputs:&[("source","object"),("recipe","object"),("projection","object")], grants:&["recipeAdopt"] },
    Endpoint { id:"prepare_execution_projection", name:"Prepare execution projection", description:"Create a disabled execution projection from an adopted source recipe.", capability:"source_execution", inputs:&[("source_id","string"),("expected_revision","string")], required:&["source_id","expected_revision"], outputs:&[("projection","object"),("readiness","object")], grants:&["executionConfigure"] },
    Endpoint { id:"refresh_execution_projection", name:"Refresh execution projection", description:"Refresh a disabled execution projection from the selected recipe.", capability:"source_execution", inputs:&[("source_id","string"),("expected_revision","string")], required:&["source_id","expected_revision"], outputs:&[("projection","object"),("readiness","object")], grants:&["executionConfigure"] },
    Endpoint { id:"get_source_readiness", name:"Get source readiness", description:"Read source health, execution alignment, test findings, blockers, and warnings.", capability:"source_execution", inputs:&[("source_id","string")], required:&["source_id"], outputs:&[("readiness","object")], grants:&[] },
    Endpoint { id:"run_safe_source_test", name:"Run safe source test", description:"Run the configured adapter without normal run outputs.", capability:"source_execution", inputs:&[("source_id","string"),("confirm_test","boolean"),("confirm_existing_session_use","boolean")], required:&["source_id"], outputs:&[("result","object"),("readiness","object"),("session","object")], grants:&["executionTest"] },
    Endpoint { id:"enable_source_when_ready", name:"Enable source when ready", description:"Enable a tested source only when existing readiness checks pass.", capability:"source_execution", inputs:&[("source_id","string"),("expected_revision","string")], required:&["source_id","expected_revision"], outputs:&[("source","object"),("readiness","object")], grants:&["executionEnable"] },
    Endpoint { id:"disable_source", name:"Disable source", description:"Disable a source execution projection without removing source history.", capability:"source_execution", inputs:&[("source_id","string"),("expected_revision","string")], required:&["source_id","expected_revision"], outputs:&[("source","object"),("projection","object")], grants:&["executionConfigure"] },
    Endpoint { id:"get_source_session_status", name:"Get source session status", description:"Read whether a source has a usable session without revealing credentials or browser control.", capability:"source_session_status", inputs:&[("source_id","string")], required:&["source_id"], outputs:&[("session","object")], grants:&[] },
];

fn field_schema(kind: &str) -> Value {
    match kind {
        "array" => json!({"type":"array","items":{"type":"object"}}),
        _ => json!({"type":kind}),
    }
}
fn object_schema(fields: Fields, required: &[&str], extra: bool) -> Value {
    let properties = fields
        .iter()
        .map(|(name, kind)| ((*name).into(), field_schema(kind)))
        .collect::<Map<String, Value>>();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":extra})
}
fn output_schema(fields: Fields) -> Value {
    let mut properties = fields
        .iter()
        .map(|(name, kind)| ((*name).into(), field_schema(kind)))
        .collect::<Map<String, Value>>();
    properties.insert("ok".into(), json!({"type":"boolean"}));
    properties.insert("message".into(), json!({"type":"string"}));
    json!({"type":"object","properties":properties,"required":["ok","message"],"additionalProperties":true})
}
fn endpoints() -> Vec<AgentMcpToolDescriptor> {
    ENDPOINTS.iter().map(|endpoint| AgentMcpToolDescriptor {
        id: endpoint.id.into(), name: endpoint.name.into(), description: endpoint.description.into(), capability: endpoint.capability.into(),
        input_schema: object_schema(endpoint.inputs, endpoint.required, false), output_schema: output_schema(endpoint.outputs),
        expected_behavior: format!("{} The endpoint returns a structured Job Agent result and applies only its declared guarded transition.", endpoint.description),
        recommended_usage: format!("Use during the {} stage when the node's instructions need this result.", CAPABILITY_GROUPS.iter().find(|group| group.id == endpoint.capability).map(|group| group.name.to_lowercase()).unwrap_or_else(|| endpoint.capability.replace('_', " "))),
        required_grants: endpoint.grants.iter().map(|value| (*value).into()).collect(),
    }).collect()
}
fn fields() -> Vec<ConfigurationField> {
    GRANTS
        .iter()
        .map(|grant| ConfigurationField {
            key: grant.id.into(),
            label: grant.label.into(),
            choices: vec!["deny".into(), "allow".into()],
            default_value: "deny".into(),
            when: None,
        })
        .collect()
}

pub(crate) fn grants_from_configuration(value: &Value) -> Result<Vec<(String, String)>, String> {
    let object = value
        .as_object()
        .ok_or("Job Agent configuration must be an object")?;
    let mut result = vec![];
    for grant in GRANTS {
        match object
            .get(grant.id)
            .and_then(Value::as_str)
            .unwrap_or("deny")
        {
            "deny" => {}
            "allow" => result.push((grant.capability.into(), grant.transition.into())),
            _ => return Err(format!("Invalid Job Agent grant value for {}", grant.id)),
        }
    }
    if let Some(key) = object
        .keys()
        .find(|key| !GRANTS.iter().any(|grant| grant.id == *key))
    {
        return Err(format!("Unknown Job Agent configuration field {key}"));
    }
    Ok(result)
}

impl OtpPackage for JobAgentPackage {
    fn descriptor(&self) -> PackageDescriptor {
        PackageDescriptor {
            id: "job_agent".into(), name: "Job Agent".into(), summary: "Guarded local source setup operations for configured agent sessions.".into(), description: "Provides a capability-scoped Job Agent MCP service for discovering sources, building recipes, and preparing controlled source execution.".into(), contract_version: 1, requested_handles: vec![], tools: vec![],
            agent_mcp_servers: vec![AgentMcpServerDescriptor {
                server_name: "job_agent".into(), name: "Job Agent MCP".into(), description: "A local MCP service delivered only to configured agent sessions, with capability-scoped endpoints and default-deny mutation grants.".into(),
                capability_groups: CAPABILITY_GROUPS.iter().map(|group| AgentMcpCapabilityGroupDescriptor { id: group.id.into(), name: group.name.into(), description: group.description.into() }).collect(),
                tools: endpoints(), grants: GRANTS.iter().map(|grant| AgentMcpGrantDescriptor { id: grant.id.into(), label: grant.label.into(), description: grant.description.into(), capability: grant.capability.into(), transition: grant.transition.into() }).collect(), configuration: fields(),
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
    #[test]
    fn grants_are_validated() {
        assert_eq!(
            grants_from_configuration(&json!({"registryConfigure":"allow"})).unwrap(),
            vec![("source_registry".into(), "configure".into())]
        );
        assert!(grants_from_configuration(&json!({"unknown":"allow"})).is_err());
    }
    #[test]
    fn catalogue_references_existing_groups_and_grants() {
        let server = JobAgentPackage.descriptor().agent_mcp_servers.remove(0);
        for tool in server.tools {
            assert!(server
                .capability_groups
                .iter()
                .any(|group| group.id == tool.capability));
            for grant in tool.required_grants {
                assert!(server.grants.iter().any(|candidate| candidate.id == grant));
            }
        }
    }
}
