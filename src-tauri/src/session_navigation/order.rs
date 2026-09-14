use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum NavigationOrderScope {
    Repositories,
    Pinned,
    Sessions { folder_id: String },
    Sections { repository_id: String },
    Workflows { repository_id: String },
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationOrder {
    pub(crate) scope: NavigationOrderScope,
    pub(crate) ordered_ids: Vec<String>,
}

pub(crate) fn validate_siblings(ids: &[String], siblings: &[String]) -> Result<(), String> {
    let unique: HashSet<_> = ids.iter().collect();
    if unique.len() != ids.len() || unique != siblings.iter().collect() {
        return Err("Order must include each current sibling exactly once".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordering_requires_current_siblings_without_duplicates() {
        let siblings = vec!["a".into(), "b".into()];
        assert!(validate_siblings(&["b".into(), "a".into()], &siblings).is_ok());
        for ids in [
            vec!["a".into()],
            vec!["a".into(), "a".into()],
            vec!["a".into(), "c".into()],
        ] {
            assert!(validate_siblings(&ids, &siblings).is_err());
        }
    }
}
