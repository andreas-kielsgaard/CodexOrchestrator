use serde_json::Value;
use std::process::{Command, Output, Stdio};

const MAX_GITHUB_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GitHubConnectionState {
    Connected,
    CliUnavailable,
    NotAuthenticated,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitHubConnection {
    pub(crate) state: GitHubConnectionState,
    pub(crate) login: Option<String>,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitHubRemoteRepository {
    pub(crate) id: String,
    pub(crate) name_with_owner: String,
    pub(crate) private: bool,
    pub(crate) web_url: String,
}

#[derive(Clone, Default)]
pub(crate) struct GitHubCatalog;

impl GitHubCatalog {
    pub(crate) fn connection(&self) -> GitHubConnection {
        let output = match gh_output([
            "auth",
            "status",
            "--active",
            "--hostname",
            "github.com",
            "--json",
            "hosts",
        ]) {
            Ok(output) => output,
            Err(_) => return connection(GitHubConnectionState::CliUnavailable, None),
        };
        let value: Value = match parse_output(&output) {
            Ok(value) => value,
            Err(_) if !output.status.success() => {
                return connection(GitHubConnectionState::NotAuthenticated, None)
            }
            Err(_) => return connection(GitHubConnectionState::Unavailable, None),
        };
        let account = value
            .pointer("/hosts/github.com")
            .and_then(Value::as_array)
            .and_then(|accounts| {
                accounts.iter().find(|account| {
                    account.get("active").and_then(Value::as_bool) == Some(true)
                        && account.get("state").and_then(Value::as_str) == Some("success")
                })
            });
        match account {
            Some(account) => connection(
                GitHubConnectionState::Connected,
                account
                    .get("login")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
            None => connection(GitHubConnectionState::NotAuthenticated, None),
        }
    }

    pub(crate) fn repositories(&self) -> Result<Vec<GitHubRemoteRepository>, String> {
        let output = gh_output([
            "api",
            "--paginate",
            "--slurp",
            "user/repos?affiliation=owner,collaborator,organization_member&per_page=100",
        ])
        .map_err(|_| "GitHub CLI is unavailable.".to_string())?;
        if !output.status.success() {
            return Err(
                "GitHub repositories could not be listed with the active CLI login.".into(),
            );
        }
        let pages: Value = parse_output(&output)?;
        let mut repositories = Vec::new();
        for value in pages
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|page| page.as_array().into_iter().flatten())
        {
            let id = value
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| "GitHub returned an invalid repository catalog.".to_string())?;
            repositories.push(GitHubRemoteRepository {
                id: format!("github-{id}"),
                name_with_owner: required_text(value, "full_name")?,
                private: value
                    .get("private")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| "GitHub returned an invalid repository catalog.".to_string())?,
                web_url: required_text(value, "html_url")?,
            });
        }
        repositories.sort_by(|left, right| {
            left.name_with_owner
                .to_ascii_lowercase()
                .cmp(&right.name_with_owner.to_ascii_lowercase())
        });
        Ok(repositories)
    }
}

fn gh_output<const N: usize>(arguments: [&str; N]) -> std::io::Result<Output> {
    Command::new("gh")
        .args(arguments)
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1")
        .env("GH_PAGER", "cat")
        .stdin(Stdio::null())
        .output()
}

fn parse_output(output: &Output) -> Result<Value, String> {
    if output.stdout.len() > MAX_GITHUB_OUTPUT_BYTES {
        return Err("GitHub returned more repositories than this operation permits.".into());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|_| "GitHub CLI returned an invalid response.".to_string())
}

fn required_text(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "GitHub returned an invalid repository catalog.".to_string())
}

fn connection(state: GitHubConnectionState, login: Option<String>) -> GitHubConnection {
    let message = match state {
        GitHubConnectionState::Connected => "GitHub CLI login is available.",
        GitHubConnectionState::CliUnavailable => "Install GitHub CLI to list remote repositories.",
        GitHubConnectionState::NotAuthenticated => {
            "Run `gh auth login` to list remote repositories."
        }
        GitHubConnectionState::Unavailable => "GitHub CLI status could not be read.",
    };
    GitHubConnection {
        state,
        login,
        message: message.into(),
    }
}
