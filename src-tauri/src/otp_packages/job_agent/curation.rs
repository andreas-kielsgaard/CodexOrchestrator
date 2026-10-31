use crate::otp_api::{tool_annotations, AgentMcpToolDescriptor};
use serde_json::{json, Map, Value};

pub(super) const TOOL_IDS: &[&str] =
    &["export_curation_dataset", "create_curation", "get_curation"];

pub(super) fn endpoints() -> Vec<AgentMcpToolDescriptor> {
    let string_array = json!({"type":["array","null"],"items":{"type":"string"}});
    let mut export = endpoint(
        TOOL_IDS[0],
        "Export curation dataset",
        "Export complete saved job evidence after structural filtering. Process the files with local scripts; do not ingest jobs.jsonl or profile.json wholesale into model context.",
        json!({
            "type":"object", "additionalProperties":false,
            "properties":{
                "job_refs":string_array, "application_statuses":string_array,
                "posting_statuses":string_array, "source_ids":string_array,
                "evidence_levels":{"type":["array","null"],"items":{"enum":["package","index_only"]}},
                "observed_from":{"type":"string"}, "observed_to":{"type":"string"}
            }
        }),
        response_schema(json!({
            "export_id":{"type":"string"}, "candidate_count":{"type":"integer"},
            "byte_count":{"type":"integer"}, "filters":{"type":"object"},
            "lifetime":{"type":"string"}, "result_uri":{"type":"string"},
            "files":{"type":"array","items":{"type":"object","properties":{
                "role":{"type":"string"}, "name":{"type":"string"},
                "media_type":{"type":"string"}, "size":{"type":"integer"},
                "sha256":{"type":"string"}, "mutable":{"type":"boolean"},
                "uri":{"type":"string"}
            }}}
        }), &["export_id","candidate_count","byte_count","filters","lifetime","result_uri","files"]),
        false,
        "Creates a host-owned temporary export and returns compact metadata and resource links. It applies only structural filters; the consuming agent decides semantic relevance.",
        "Use a local session whose writable workspace contains the Job Agent export directory. Stream the files with local scripts and reveal only bounded derived evidence. Exports last for the MCP host lifetime and are local to that host; remote file access is not provided.",
    );
    export.meta.insert(
        "com.orchid/resultHandling".into(),
        json!({
            "disposition":"artifact-required", "modelIngestion":"avoid",
            "contentTypes":["application/x-ndjson","application/json"], "cardinality":"many"
        }),
    );
    vec![
        export,
        endpoint(
            TOOL_IDS[1], "Create curation",
            "Validate the fixed curation-result.json in a prior export and persist an advisory curation.",
            id_schema("export_id"),
            response_schema(json!({
                "curation_id":{"type":"string"}, "export_id":{"type":"string"},
                "title":{"type":"string"}, "screened_count":{"type":"integer"},
                "curated_count":{"type":"integer"}, "summary":{"type":"string"},
                "view_url":{"type":"string"}
            }), &["curation_id","export_id","title","screened_count","curated_count","summary","view_url"]),
            false,
            "Verifies the export and validates the result before saving it. Does not change application statuses or deterministic scores. Validation failures save nothing and include error details.",
            "Write curation-result.json at the export's result_uri using its result schema and exact job refs. On a validation error, edit that file and retry with the same export_id while the same MCP host is alive.",
        ),
        endpoint(
            TOOL_IDS[2], "Get curation", "Read a saved advisory curation by ID.",
            id_schema("curation_id"),
            response_schema(json!({"curation":{"type":"object"}}), &["curation"]),
            true,
            "Reads a durable saved curation, or returns curation_not_found. Reading does not modify jobs or source configuration.",
            "Use the curation_id returned by Create curation to inspect the saved result. The saved curation survives temporary export cleanup.",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn endpoint(
    id: &str,
    name: &str,
    description: &str,
    input_schema: Value,
    output_schema: Value,
    read_only: bool,
    expected_behavior: &str,
    recommended_usage: &str,
) -> AgentMcpToolDescriptor {
    AgentMcpToolDescriptor {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        capability: "curation".into(),
        input_schema,
        output_schema,
        expected_behavior: expected_behavior.into(),
        recommended_usage: recommended_usage.into(),
        required_grants: vec![],
        annotations: tool_annotations(name, read_only, false, read_only, false),
        meta: Map::new(),
    }
}

fn id_schema(field: &str) -> Value {
    json!({"type":"object","properties":{field:{"type":"string"}},"required":[field],"additionalProperties":false})
}

fn response_schema(fields: Value, required: &[&str]) -> Value {
    let mut properties = fields.as_object().unwrap().clone();
    properties.insert("schema_version".into(), json!({"const":1}));
    properties.insert("ok".into(), json!({"type":"boolean"}));
    properties.insert("message".into(), json!({"type":"string"}));
    properties.insert(
        "error".into(),
        json!({
            "type":"object", "required":["code","message"],
            "properties":{"code":{"type":"string"},"message":{"type":"string"},
                "issue_code":{"type":"string"},"path":{"type":"string"}}
        }),
    );
    let mut success_required = vec!["message"];
    success_required.extend_from_slice(required);
    json!({
        "type":"object", "properties":properties, "required":["schema_version","ok"],
        "if":{"properties":{"ok":{"const":true}}},
        "then":{"required":success_required}, "else":{"required":["error"]}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_contract_uses_structural_filters_and_artifact_metadata() {
        let tools = endpoints();
        let export = &tools[0];
        assert_eq!(
            export.input_schema["properties"]["job_refs"]["items"]["type"],
            "string"
        );
        assert!(export.input_schema["properties"].get("keywords").is_none());
        assert_eq!(
            export.meta["com.orchid/resultHandling"]["disposition"],
            "artifact-required"
        );
        assert_eq!(export.output_schema["else"]["required"], json!(["error"]));
        for (index, tool) in tools.iter().enumerate() {
            assert_eq!(tool.capability, "curation");
            assert!(tool.required_grants.is_empty());
            assert_eq!(
                serde_json::to_value(&tool.annotations).unwrap()["readOnlyHint"],
                index == 2
            );
        }
        assert_eq!(tools[1].input_schema["required"], json!(["export_id"]));
        assert!(tools[1].input_schema["properties"]
            .get("result_file")
            .is_none());
    }
}
