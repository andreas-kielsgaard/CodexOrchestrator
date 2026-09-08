use super::{compiled_plan::FileAssociation, instances::RecipeInstance};

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::session_events::SessionLogicalAddress;

    #[test]
    fn windows_file_paths_match_worktree_case_and_component_boundaries() {
        let root = Path::new("C:/Work/Project");
        let mut change = SessionFileChange {
            address: SessionLogicalAddress::new(
                ReferenceIdentity::new("workflow", "instance", "run").unwrap(),
                ReferenceIdentity::new("workflow", "node", "author").unwrap(),
            ),
            session_id: "session".into(),
            invocation_id: "invocation".into(),
            working_directory: Some("c:/work/project/docs".into()),
            path: "c:/work/project/docs/plan.md".into(),
            operation: FileOperation::Edit,
            recorded_at: String::new(),
        };
        assert_eq!(relative_path(root, &change), Some("docs/plan.md".into()));
        change.path = "../plan.md".into();
        assert_eq!(relative_path(root, &change), Some("plan.md".into()));
        change.path = "C:/Work/ProjectElsewhere/plan.md".into();
        assert_eq!(relative_path(root, &change), None);
        change.path = "../../outside.md".into();
        assert_eq!(relative_path(root, &change), None);
    }
}

use crate::agent_sessions::{
    file_history::{FileOperation, SessionFileChange},
    ports::AgentSessionRepository,
};
use crate::session_events::ReferenceIdentity;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

pub(crate) fn resolve_node_files(
    instance: &RecipeInstance,
    node_id: &str,
    association: FileAssociation,
    sessions: &dyn AgentSessionRepository,
) -> Result<String, String> {
    if !instance
        .recipe
        .nodes
        .iter()
        .any(|node| node.node_id == node_id)
    {
        return Err("File input node is absent from the instance recipe".into());
    }
    let scope = ReferenceIdentity::new("workflow", "instance", &instance.id)
        .map_err(|error| error.to_string())?;
    let history = sessions
        .file_history_at_scope(&scope)
        .map_err(|error| error.to_string())?;
    let root = Path::new(&instance.target.worktree.path);
    let mut files: BTreeMap<String, FileSummary> = BTreeMap::new();
    for change in &history {
        if change.address.subject.namespace() != "workflow"
            || change.address.subject.kind() != "node"
        {
            continue;
        }
        let Some(path) = relative_path(root, change) else {
            continue;
        };
        let key = if cfg!(windows) {
            path.to_lowercase()
        } else {
            path.clone()
        };
        let file = files.entry(key).or_insert_with(|| FileSummary {
            path,
            matched: false,
            created: None,
            edited: None,
        });
        if change.address.subject.id() == node_id {
            file.matched |= matches!(
                (association, change.operation),
                (
                    FileAssociation::Created | FileAssociation::Either,
                    FileOperation::Create
                ) | (
                    FileAssociation::Edited | FileAssociation::Either,
                    FileOperation::Edit
                )
            );
        }
        match change.operation {
            FileOperation::Create => {
                file.created = Some(attribution(instance, change));
                file.edited = None;
            }
            FileOperation::Edit => file.edited = Some(attribution(instance, change)),
            FileOperation::Delete => {}
        }
    }
    let files: Vec<Value> = files.into_values().filter(|file| file.matched).map(|file| {
        json!({"path": file.path, "exists": root.join(&file.path).is_file(), "created": file.created, "lastEdited": file.edited})
    }).collect();
    serde_json::to_string_pretty(&json!({"nodeId":node_id,"nodeName":instance.recipe.nodes.iter().find(|node|node.node_id==node_id).map(|node|&node.name),"association":association,"files":files})).map_err(|error| error.to_string())
}

struct FileSummary {
    path: String,
    matched: bool,
    created: Option<Value>,
    edited: Option<Value>,
}

fn attribution(instance: &RecipeInstance, change: &SessionFileChange) -> Value {
    let node_id = change.address.subject.id();
    json!({"nodeId":node_id,"nodeName":instance.recipe.nodes.iter().find(|node|node.node_id==node_id).map(|node|&node.name),
        "sessionId":change.session_id,"invocationId":change.invocation_id,"recordedAt":change.recorded_at})
}

fn relative_path(root: &Path, change: &SessionFileChange) -> Option<String> {
    let path = Path::new(&change.path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(change.working_directory.as_deref()?).join(path)
    };
    let absolute = normalize(&absolute);
    let root = normalize(root);
    let mut parts = absolute.components();
    for expected in root.components() {
        let actual = parts.next()?;
        let same = if cfg!(windows) {
            actual
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&expected.as_os_str().to_string_lossy())
        } else {
            actual == expected
        };
        if !same {
            return None;
        }
    }
    let relative = parts.as_path();
    if relative.as_os_str().is_empty() {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}
