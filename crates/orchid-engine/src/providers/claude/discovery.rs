//! What a Claude setup offers: models and effort levels from Claude's `initialize` report, and
//! skills from the setup's and the project's skill folders.
use crate::contracts::{ProviderSkill, ProviderSkillCatalogue};
use serde::Deserialize;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeModel {
    /// The value `--model` accepts, such as an alias or a full model ID.
    pub value: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub supported_effort_levels: Vec<String>,
}

/// Starts `claude`, asks for its `initialize` report and closes its input, which ends the
/// process without starting a conversation.
pub fn models(program: &str, environment: &[(String, String)]) -> Result<Vec<ClaudeModel>, String> {
    let mut command = Command::new(program);
    command
        .args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
        ])
        .envs(environment.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for variable in super::launch::parent_session_variables() {
        command.env_remove(variable);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Claude Code CLI could not start: {error}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let written = stdin.write_all(
        b"{\"type\":\"control_request\",\"request_id\":\"orchid-models\",\"request\":{\"subtype\":\"initialize\"}}\n",
    );
    drop(stdin);
    let reply = written.map_err(|error| error.to_string()).and_then(|()| {
        BufReader::new(child.stdout.take().expect("piped stdout"))
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
            .find(|message| message["response"]["request_id"] == "orchid-models")
            .ok_or_else(|| "Claude Code did not report its models".to_string())
    });
    let _ = child.wait();
    let reply = reply?;
    if reply["response"]["subtype"] != "success" {
        return Err(reply["response"]["error"]
            .as_str()
            .unwrap_or("Claude Code did not report its models")
            .into());
    }
    serde_json::from_value(reply["response"]["response"]["models"].clone())
        .map_err(|error| format!("Claude Code reported unreadable models: {error}"))
}

/// Skills in `<config folder>/skills` and `<project>/.claude/skills`, one folder per skill.
pub fn skills(config_folder: &Path, project: Option<&Path>) -> ProviderSkillCatalogue {
    let mut catalogue = ProviderSkillCatalogue::default();
    let roots = std::iter::once((config_folder.join("skills"), "user"))
        .chain(project.map(|project| (project.join(".claude").join("skills"), "project")));
    for (root, scope) in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut found: Vec<_> = entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path().join("SKILL.md");
                let content = std::fs::read_to_string(&path).ok()?;
                Some(ProviderSkill {
                    name: entry.file_name().to_str()?.to_owned(),
                    description: description(&content),
                    path: path.to_string_lossy().into_owned(),
                    scope: scope.into(),
                    enabled: true,
                })
            })
            .collect();
        found.sort_by(|left, right| left.name.cmp(&right.name));
        catalogue.skills.extend(found);
    }
    catalogue
}

/// The `description:` line of a skill's front matter.
fn description(content: &str) -> String {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return String::new();
    }
    lines
        .take_while(|line| line.trim() != "---")
        .find_map(|line| line.strip_prefix("description:"))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_setup_and_project_skills_with_their_descriptions() {
        let config = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let write = |root: &Path, name: &str, content: &str| {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join("SKILL.md"), content).unwrap();
        };
        write(
            &config.path().join("skills"),
            "review",
            "---\nname: review\ndescription: \"Review a diff\"\n---\nBody",
        );
        write(
            &project.path().join(".claude/skills"),
            "deploy",
            "No front matter",
        );
        std::fs::create_dir_all(config.path().join("skills/empty")).unwrap();
        let catalogue = skills(config.path(), Some(project.path()));
        let found: Vec<_> = catalogue
            .skills
            .iter()
            .map(|skill| {
                (
                    skill.name.as_str(),
                    skill.scope.as_str(),
                    skill.description.as_str(),
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("review", "user", "Review a diff"),
                ("deploy", "project", "")
            ]
        );
    }

    #[test]
    fn reads_models_from_the_recorded_initialize_report() {
        let report = super::super::tests::fixture("initialize")
            .into_iter()
            .find(|message| message["type"] == "control_response")
            .unwrap();
        let models: Vec<ClaudeModel> =
            serde_json::from_value(report["response"]["response"]["models"].clone()).unwrap();
        let sonnet = models.iter().find(|model| model.value == "sonnet").unwrap();
        assert_eq!(sonnet.display_name, "Sonnet 5");
        assert!(sonnet.supported_effort_levels.contains(&"high".to_string()));
        assert!(models
            .iter()
            .any(|model| model.value == "haiku" && model.supported_effort_levels.is_empty()));
    }
}
