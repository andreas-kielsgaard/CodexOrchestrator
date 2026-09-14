use super::{
    command::{HardenedGitRunner, SMALL_OUTPUT_LIMIT},
    invalid_output, RepositoryContextError,
};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteObservation {
    pub name: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

#[derive(Clone)]
pub struct RepositoryRemoteReader {
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryRemoteReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub fn list(
        &self,
        repository_root: &Path,
    ) -> Result<Vec<RemoteObservation>, RepositoryContextError> {
        let bytes = self
            .runner
            .required(repository_root, ["remote", "-v"], SMALL_OUTPUT_LIMIT)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| invalid_output())?;
        parse_remote_output(text)
    }
}

fn parse_remote_output(text: &str) -> Result<Vec<RemoteObservation>, RepositoryContextError> {
    let mut remotes = Vec::<RemoteObservation>::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let name = fields.next().ok_or_else(invalid_output)?;
        let url = fields.next().ok_or_else(invalid_output)?;
        let direction = fields.next().ok_or_else(invalid_output)?;
        if fields.next().is_some() || name.is_empty() || url.is_empty() {
            return Err(invalid_output());
        }
        let remote = if let Some(remote) = remotes.iter_mut().find(|remote| remote.name == name) {
            remote
        } else {
            remotes.push(RemoteObservation {
                name: name.to_owned(),
                fetch_url: None,
                push_url: None,
            });
            remotes.last_mut().expect("just inserted")
        };
        match direction {
            "(fetch)" => remote.fetch_url = Some(url.to_owned()),
            "(push)" => remote.push_url = Some(url.to_owned()),
            _ => return Err(invalid_output()),
        }
    }
    Ok(remotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_fetch_and_push_urls_without_assigning_identity() {
        let remotes = parse_remote_output(
            "origin\tgit@github.com:openai/codex.git (fetch)\norigin\thttps://github.com/openai/codex.git (push)\n",
        )
        .unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(
            remotes[0].fetch_url.as_deref(),
            Some("git@github.com:openai/codex.git")
        );
    }
}
