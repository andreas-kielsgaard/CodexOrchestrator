use super::instances::RecipeInstance;
use crate::session_events::{
    PromptSourceDefinition, ReferenceIdentity, SessionEventDefinition, SessionEventOccurrence,
};
use std::{fs, path::Path};

/// Only explicit file references are read, relative to this instance's chosen worktree.
pub(crate) fn supply_referenced_content(
    instance: &RecipeInstance,
    definition: &SessionEventDefinition,
    occurrence: &mut SessionEventOccurrence,
) -> Result<(), String> {
    for source in definition
        .prompt_sources
        .iter()
        .chain(&definition.created_session_prompt_sources)
    {
        let PromptSourceDefinition::ReferencedContent { reference } = source else {
            continue;
        };
        if reference.namespace() == "workflow" && reference.kind() == "prompt_field" {
            let (recipe_id, revision, owner, field): (String, u64, String, String) =
                serde_json::from_str(reference.id()).map_err(|error| error.to_string())?;
            if recipe_id != instance.recipe.recipe_id || revision != instance.recipe.revision {
                return Err("Prompt field belongs to a different recipe snapshot".into());
            }
            let content = match field.as_str() {
                "initialPrompt" => instance
                    .recipe
                    .nodes
                    .iter()
                    .find(|node| node.node_id == owner)
                    .and_then(|node| node.initial_prompt.clone()),
                "promptText" => instance
                    .recipe
                    .connections
                    .iter()
                    .find(|edge| edge.connection_id == owner)
                    .map(|edge| edge.prompt_text.clone()),
                _ => None,
            }
            .ok_or("Prompt field is absent from the instance recipe")?;
            occurrence
                .referenced_content
                .insert(reference.clone(), content);
            continue;
        }
        if reference.namespace() != "file" || reference.kind() != "path" {
            return Err(format!(
                "Unsupported prompt file reference `{reference}`; use file/path"
            ));
        }
        let root = Path::new(&instance.target.worktree.path)
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let path = root
            .join(reference.id())
            .canonicalize()
            .map_err(|error| format!("Cannot read prompt file `{}`: {error}", reference.id()))?;
        if !path.starts_with(&root) {
            return Err("Prompt file is outside the instance worktree".into());
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("Cannot read prompt file: {error}"))?;
        occurrence
            .referenced_content
            .insert(reference.clone(), text);
    }
    Ok(())
}

pub(crate) fn reference_fixed_prompts(
    instance: &RecipeInstance,
    definitions: &mut [SessionEventDefinition],
) -> Result<(), String> {
    for (index, definition) in definitions.iter_mut().enumerate() {
        let node = if index == 0 {
            instance.recipe.starting_node_id.as_deref()
        } else {
            instance
                .recipe
                .connections
                .get(index - 1)
                .map(|connection| connection.destination_node_id.as_str())
        };
        let field_reference =
            |owner: &str, field: &str| -> Result<PromptSourceDefinition, String> {
                Ok(PromptSourceDefinition::ReferencedContent {
                    reference: ReferenceIdentity::new(
                        "workflow",
                        "prompt_field",
                        serde_json::to_string(&(
                            &instance.recipe.recipe_id,
                            instance.recipe.revision,
                            owner,
                            field,
                        ))
                        .map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?,
                })
            };
        if let Some(node) = node {
            for source in &mut definition.created_session_prompt_sources {
                if matches!(source, PromptSourceDefinition::Literal { .. }) {
                    *source = field_reference(node, "initialPrompt")?;
                }
            }
        }
        if let Some(connection) = index
            .checked_sub(1)
            .and_then(|index| instance.recipe.connections.get(index))
        {
            for source in &mut definition.prompt_sources {
                if matches!(source, PromptSourceDefinition::Literal { .. }) {
                    *source = field_reference(&connection.connection_id, "promptText")?;
                }
            }
        }
    }
    Ok(())
}
