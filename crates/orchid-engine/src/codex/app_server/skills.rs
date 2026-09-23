//! Codex-owned skill discovery for a registered home and working context.
use super::connection::Connection;
use crate::contracts::ports::RuntimePortError;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSkill {
    pub name: String,
    pub description: String,
    pub path: String,
    pub scope: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSkillCatalogue {
    pub skills: Vec<CodexSkill>,
    pub limitations: Vec<String>,
}

pub(super) fn read(
    connection: &Connection,
    cwd: &Path,
) -> Result<CodexSkillCatalogue, RuntimePortError> {
    let response = read_response(connection, cwd, false)?;
    Ok(project(&response))
}

pub(super) fn read_forced(
    connection: &Connection,
    cwd: &Path,
) -> Result<CodexSkillCatalogue, RuntimePortError> {
    let response = read_response(connection, cwd, true)?;
    Ok(project(&response))
}

pub(super) fn read_response(
    connection: &Connection,
    cwd: &Path,
    force_reload: bool,
) -> Result<Value, RuntimePortError> {
    connection.call(
        "skills/list",
        json!({"cwds":[cwd],"forceReload":force_reload}),
    )
}

pub fn project(response: &Value) -> CodexSkillCatalogue {
    let mut catalogue = CodexSkillCatalogue::default();
    if !response["data"].is_array() {
        catalogue
            .limitations
            .push("Codex returned an invalid skill catalogue.".into());
        return catalogue;
    }
    for group in response["data"].as_array().into_iter().flatten() {
        if let Some(errors) = group["errors"].as_array() {
            for error in errors {
                catalogue.limitations.push(
                    error["message"]
                        .as_str()
                        .unwrap_or("Codex reported a skill discovery error")
                        .into(),
                );
            }
        }
        for skill in group["skills"].as_array().into_iter().flatten() {
            let (Some(name), Some(path)) = (skill["name"].as_str(), skill["path"].as_str()) else {
                continue;
            };
            catalogue.skills.push(CodexSkill {
                name: name.into(),
                description: skill["interface"]["shortDescription"]
                    .as_str()
                    .or(skill["description"].as_str())
                    .unwrap_or("")
                    .into(),
                path: path.into(),
                scope: skill["scope"].as_str().unwrap_or("native").into(),
                enabled: skill["enabled"] != false,
            });
        }
    }
    catalogue
        .skills
        .sort_by(|a, b| (&a.name, &a.path).cmp(&(&b.name, &b.path)));
    catalogue
        .skills
        .dedup_by(|a, b| a.name == b.name && a.path == b.path);
    catalogue
}

pub fn mentioned<'a>(text: &str, catalogue: &'a CodexSkillCatalogue) -> Vec<&'a CodexSkill> {
    let mut counts = std::collections::HashMap::new();
    for skill in catalogue.skills.iter().filter(|skill| skill.enabled) {
        *counts.entry(skill.name.as_str()).or_insert(0usize) += 1;
    }
    catalogue
        .skills
        .iter()
        .filter(|skill| skill.enabled && counts.get(skill.name.as_str()) == Some(&1))
        .filter(|skill| {
            let marker = format!("${}", skill.name);
            text.match_indices(&marker).any(|(start, _)| {
                text[start + marker.len()..]
                    .chars()
                    .next()
                    .is_none_or(|next| !next.is_alphanumeric() && next != '_' && next != '-')
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_paths_outside_codex_home_and_reports_errors() {
        let result = project(&json!({"data":[{"skills":[
            {"name":"repo","path":"/repo/.agents/skills/repo/SKILL.md","scope":"repo"},
            {"name":"disabled","path":"/elsewhere/SKILL.md","enabled":false}
        ],"errors":[{"message":"another root failed"}]}]}));
        assert_eq!(result.skills.len(), 2);
        assert_eq!(result.skills[1].scope, "repo");
        assert!(!result.skills[0].enabled);
        assert_eq!(result.limitations, ["another root failed"]);
    }
}
