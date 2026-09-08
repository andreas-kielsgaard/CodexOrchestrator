use super::{
    compiled_plan::{WorkflowCompiledConnection, WorkflowConnectionPromptInput},
    instances::RecipeInstance,
};
use crate::{agent_sessions::ports::AgentSessionRepository, otp_api::ResolvedInput};
use serde_json::Value;
use std::{fs, path::Path};

pub(crate) fn resolve_inputs(
    instance: &RecipeInstance,
    connection: &WorkflowCompiledConnection,
    payload: &Value,
    sessions: &dyn AgentSessionRepository,
) -> Result<Vec<ResolvedInput>, String> {
    let mut inputs = Vec::new();
    for (ordinal, input) in connection.prompt_inputs.iter().enumerate() {
        let value = match input {
            WorkflowConnectionPromptInput::OutputField { field } => payload
                .get(field)
                .cloned()
                .ok_or_else(|| format!("Output did not offer field {field}"))?,
            WorkflowConnectionPromptInput::NodeFiles {
                node_id,
                association,
            } => serde_json::from_str(&super::file_inputs::resolve_node_files(
                instance,
                node_id,
                *association,
                sessions,
            )?)
            .map_err(|e| e.to_string())?,
            WorkflowConnectionPromptInput::FileContent { path } => {
                let root = Path::new(&instance.target.worktree.path)
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                let file = root
                    .join(path)
                    .canonicalize()
                    .map_err(|e| format!("Cannot read prompt file {path}: {e}"))?;
                if !file.starts_with(&root) {
                    return Err("Prompt file is outside the instance worktree".into());
                }
                Value::String(fs::read_to_string(file).map_err(|e| e.to_string())?)
            }
        };
        inputs.push(ResolvedInput {
            reference: serde_json::to_string(&(
                &instance.recipe.recipe_id,
                instance.recipe.revision,
                connection.reference.identity(),
                ordinal,
                input,
            ))
            .map_err(|e| e.to_string())?,
            value,
        });
    }
    if !connection.prompt_text.trim().is_empty() {
        inputs.push(ResolvedInput {
            reference: serde_json::to_string(&(
                &instance.recipe.recipe_id,
                instance.recipe.revision,
                connection.reference.identity(),
                "promptText",
            ))
            .map_err(|e| e.to_string())?,
            value: Value::String(connection.prompt_text.clone()),
        });
    }
    Ok(inputs)
}
