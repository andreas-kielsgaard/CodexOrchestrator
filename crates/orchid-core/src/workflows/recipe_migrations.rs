use super::authoring::WorkflowRecipeDraft;
use serde_json::{json, Map, Value};

pub const WORKFLOW_RECIPE_CONTRACT_VERSION: u32 = 4;

/// Reads persisted recipes into the current contract. The next save persists V4.
pub fn decode_recipe_value(value: Value) -> Result<WorkflowRecipeDraft, String> {
    let value = match value.get("contractVersion").and_then(Value::as_u64) {
        Some(1) => migrate_v3_recipe(migrate_v1_recipe(value)?)?,
        Some(2) => migrate_v3_recipe(migrate_v2_recipe(value)?)?,
        Some(3) => migrate_v3_recipe(value)?,
        Some(version) if version == WORKFLOW_RECIPE_CONTRACT_VERSION as u64 => value,
        Some(version) => {
            return Err(format!(
                "Workflow recipe contract version {version} is unsupported"
            ))
        }
        None => return Err("Workflow recipe has no contractVersion".into()),
    };
    let draft: WorkflowRecipeDraft = serde_json::from_value(value)
        .map_err(|error| format!("Unable to decode Workflow recipe: {error}"))?;
    draft.validate_storable()?;
    Ok(draft)
}

pub fn decode_recipe_state(
    mut value: Value,
) -> Result<super::authoring::WorkflowRecipeState, String> {
    let state = value
        .as_object_mut()
        .ok_or("Workflow state must be an object")?;
    for key in ["draft", "active"] {
        if let Some(recipe) = state.get_mut(key).filter(|recipe| !recipe.is_null()) {
            *recipe = serde_json::to_value(decode_recipe_value(recipe.take())?)
                .map_err(|error| error.to_string())?;
        }
    }
    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn migrate_v1_recipe(mut value: Value) -> Result<Value, String> {
    let recipe = value
        .as_object_mut()
        .ok_or("Workflow recipe must be an object")?;
    recipe.insert("contractVersion".into(), json!(2));
    recipe.insert(
        "entryAction".into(),
        json!({"package":"workflow","tool":"prompt_agent"}),
    );
    recipe.insert("entryConfiguration".into(), json!({}));
    for node in required_array_mut(recipe, "nodes")? {
        node.as_object_mut()
            .ok_or("Workflow node must be an object")?
            .entry("agentMcpConfiguration")
            .or_insert_with(|| json!({}));
    }
    let connections = required_array_mut(recipe, "connections")?;
    for connection in connections {
        migrate_v1_connection(connection)?;
    }
    migrate_v2_recipe(value)
}

fn migrate_v2_recipe(mut value: Value) -> Result<Value, String> {
    let recipe = value
        .as_object_mut()
        .ok_or("Workflow recipe must be an object")?;
    recipe.insert(
        "contractVersion".into(),
        json!(WORKFLOW_RECIPE_CONTRACT_VERSION),
    );
    let entry_nodes = recipe
        .remove("startingNodeId")
        .and_then(|value| value.as_str().map(str::to_owned))
        .into_iter()
        .map(Value::String)
        .collect();
    recipe.insert("entryNodeIds".into(), Value::Array(entry_nodes));
    recipe.remove("entryAction");
    recipe.remove("entryConfiguration");
    for node in required_array_mut(recipe, "nodes")? {
        node.as_object_mut()
            .ok_or("Workflow node must be an object")?
            .remove("agentMcpConfiguration");
    }
    Ok(value)
}

fn required_array_mut<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Vec<Value>, String> {
    object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("Workflow recipe {key} must be an array"))
}

fn migrate_v1_connection(value: &mut Value) -> Result<(), String> {
    let connection = value
        .as_object_mut()
        .ok_or("Workflow connection must be an object")?;
    let trigger = connection
        .remove("trigger")
        .ok_or("Workflow connection has no trigger")?;
    let target = connection.remove("target").unwrap_or_else(|| json!({}));
    let prompt_inputs = connection
        .remove("promptInputs")
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .map(migrate_v1_prompt_input)
        .collect::<Result<Vec<_>, _>>()?;
    connection.insert("trigger".into(), migrate_v1_trigger(trigger)?);
    connection.insert(
        "action".into(),
        json!({"package":"workflow","tool":"prompt_agent"}),
    );
    connection.insert("configuration".into(), v1_target_configuration(&target));
    connection.insert("promptInputs".into(), Value::Array(prompt_inputs));
    Ok(())
}

fn migrate_v1_trigger(value: Value) -> Result<Value, String> {
    let trigger = value
        .as_object()
        .ok_or("Workflow trigger must be an object")?;
    let kind = trigger
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tool = match kind {
        "invocation_completed" => "on_invocation_completed",
        "mcp_call" => match trigger
            .get("tool")
            .and_then(|tool| tool.get("id"))
            .and_then(Value::as_str)
        {
            Some("handoff_to_agent") => "handoff_to_agent",
            Some("trigger_workflow_continuation") => "trigger_workflow_continuation",
            Some(tool) => return Err(format!("Cannot migrate Workflow MCP trigger `{tool}`")),
            None => return Err("Cannot migrate Workflow MCP trigger without a tool id".into()),
        },
        "application_event" | "event_group_completed" => {
            return Err(format!(
                "Cannot migrate Workflow trigger kind `{kind}` to an OTP output"
            ))
        }
        _ => {
            return Err(format!(
                "Cannot migrate unknown Workflow trigger kind `{kind}`"
            ))
        }
    };
    let output = match tool {
        "on_invocation_completed" => "completed",
        "handoff_to_agent" => "handoff",
        "trigger_workflow_continuation" => "continuation",
        _ => unreachable!(),
    };
    Ok(json!({"capability":{"package":"workflow","tool":tool},"output":output}))
}

fn migrate_v1_prompt_input(value: Value) -> Result<Value, String> {
    let input = value
        .as_object()
        .ok_or("Workflow prompt input must be an object")?;
    let kind = input
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "invocation_output" => Ok(json!({"kind":"output_field","field":"output"})),
        "mcp_argument" => Ok(json!({
            "kind":"output_field",
            "field": input.get("name").and_then(Value::as_str).unwrap_or_default(),
        })),
        "application_event_field" => Ok(json!({
            "kind":"output_field",
            "field": input.get("field").and_then(Value::as_str).unwrap_or_default(),
        })),
        "referenced_content" => Ok(json!({
            "kind":"file_content",
            "path": input.get("reference").and_then(|reference| reference.get("id")).and_then(Value::as_str).unwrap_or_default(),
        })),
        _ => Err(format!(
            "Cannot migrate Workflow prompt input kind `{kind}`"
        )),
    }
}

fn v1_target_configuration(target: &Value) -> Value {
    let source = target.as_object();
    let value = |key: &str, fallback: &str| {
        source
            .and_then(|target| target.get(key))
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    };
    json!({
        "mode":"select",
        "cardinality":value("cardinality", "first"),
        "ordering":value("ordering", "newest"),
        "running":value("running", "any"),
        "missing":value("missing", "create"),
    })
}

fn migrate_v3_recipe(mut value: Value) -> Result<Value, String> {
    let recipe = value
        .as_object_mut()
        .ok_or("Workflow recipe must be an object")?;
    recipe.insert(
        "contractVersion".into(),
        json!(WORKFLOW_RECIPE_CONTRACT_VERSION),
    );
    for node in required_array_mut(recipe, "nodes")? {
        node.as_object_mut()
            .ok_or("Workflow node must be an object")?
            .insert("kind".into(), json!("agent"));
    }
    Ok(value)
}
