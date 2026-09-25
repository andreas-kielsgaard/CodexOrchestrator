//! Codex-owned skill discovery for a registered home and working context.
use super::connection::Connection;
use crate::contracts::{ports::RuntimePortError, ProviderSkill, ProviderSkillCatalogue};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn read(
    connection: &Connection,
    cwd: &Path,
) -> Result<ProviderSkillCatalogue, RuntimePortError> {
    let response = read_response(connection, cwd, false)?;
    Ok(project(&response))
}

pub(super) fn read_forced(
    connection: &Connection,
    cwd: &Path,
) -> Result<ProviderSkillCatalogue, RuntimePortError> {
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

pub fn project(response: &Value) -> ProviderSkillCatalogue {
    let mut catalogue = ProviderSkillCatalogue::default();
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
            catalogue.skills.push(ProviderSkill {
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
