use super::branch_graph::{self, BranchGraphView, GraphConnection};
use super::{
    branch_presentation::{commit_view, BranchView, CommitView},
    domain::ReviewTarget,
};
use crate::repository_context::{ObjectId, RepositoryContext, RepositoryIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HistoryScope {
    Ancestry {
        tip_object_id: String,
        excluded_base_object_id: Option<String>,
    },
    GraphRange {
        snapshot_id: String,
        range_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitHistoryQuery {
    pub(crate) target: ReviewTarget,
    pub(crate) scope: HistoryScope,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitHistoryPageView {
    pub(crate) scope: HistoryScope,
    pub(crate) total_count: usize,
    pub(crate) commits: Vec<CommitView>,
    pub(crate) next_cursor: Option<String>,
}

const EXPIRED: &str = "This graph snapshot expired; reopen the branch selector.";
struct StoredRange {
    members: Vec<ObjectId>,
    sources: Vec<ReviewTarget>,
}
struct Snapshot {
    id: String,
    repository_id: String,
    targets: Vec<BranchView>,
    reference_target: Option<ReviewTarget>,
    ranges: HashMap<String, StoredRange>,
}
#[derive(Default)]
pub(crate) struct BranchHistoryService {
    snapshots: Mutex<VecDeque<Snapshot>>,
}

impl BranchHistoryService {
    pub(super) fn graph(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        limit: usize,
        snapshot_id: Option<&str>,
        inventory: impl FnOnce() -> Result<Vec<BranchView>, String>,
    ) -> Result<BranchGraphView, String> {
        let (id, targets, reference_target) = if let Some(id) = snapshot_id {
            let snapshots = self
                .snapshots
                .lock()
                .map_err(|_| "Graph snapshots unavailable")?;
            let snapshot = snapshots
                .iter()
                .find(|s| s.id == id && s.repository_id == repository.id.as_str())
                .ok_or(EXPIRED)?;
            (
                snapshot.id.clone(),
                snapshot.targets.clone(),
                snapshot.reference_target.clone(),
            )
        } else {
            let targets = inventory()?;
            let default = context
                .references()
                .remote_default_branch(repository.top_level.path())
                .map_err(|e| e.to_string())?;
            let default_local = default
                .as_ref()
                .and_then(|r| r.as_str().strip_prefix("refs/remotes/origin/"))
                .map(|name| format!("refs/heads/{name}"));
            let reference_target = targets
                .iter()
                .find(|t| default_local.is_some() && t.branch_ref == default_local)
                .or_else(|| targets.iter().find(|t| t.display_name == "main"))
                .or_else(|| targets.iter().find(|t| t.display_name == "master"))
                .or_else(|| targets.iter().min_by_key(|t| &t.display_name))
                .map(|t| t.target.clone());
            let id = uuid::Uuid::new_v4().to_string();
            let mut snapshots = self
                .snapshots
                .lock()
                .map_err(|_| "Graph snapshots unavailable")?;
            snapshots.push_back(Snapshot {
                id: id.clone(),
                repository_id: repository.id.as_str().into(),
                targets: targets.clone(),
                reference_target: reference_target.clone(),
                ranges: HashMap::new(),
            });
            while snapshots.len() > 8 {
                snapshots.pop_front();
            }
            (id, targets, reference_target)
        };
        let tips = targets
            .iter()
            .map(|t| ObjectId::parse(&t.tip.object_id).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let (commits, has_more) = context
            .commits()
            .graph_page(repository.top_level.path(), &tips, limit)
            .map_err(|e| e.to_string())?;
        let (anchors, ranges) = branch_graph::project(
            &commits,
            &targets
                .iter()
                .map(|t| t.tip.object_id.clone())
                .collect::<Vec<_>>(),
        );
        let mut snapshots = self
            .snapshots
            .lock()
            .map_err(|_| "Graph snapshots unavailable")?;
        let snapshot = snapshots.iter_mut().find(|s| s.id == id).ok_or(EXPIRED)?;
        let mut connections = Vec::new();
        for range in ranges {
            let range_id = range.id();
            let sources: Vec<_> = range
                .sources
                .iter()
                .map(|&i| targets[i].target.clone())
                .collect();
            let members = range
                .members
                .iter()
                .map(|id| ObjectId::parse(id).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            connections.push(GraphConnection {
                id: range_id.clone(),
                from: range.from,
                to: range.to,
                commit_count: members.len(),
                collapsed: range.collapsed,
                incomplete: range.incomplete,
                eligible_sources: sources.clone(),
                scope: HistoryScope::GraphRange {
                    snapshot_id: id.clone(),
                    range_id: range_id.clone(),
                },
            });
            snapshot
                .ranges
                .entry(range_id)
                .or_insert(StoredRange { members, sources });
        }
        let has_more = has_more
            && (anchors.iter().any(|a| a.boundary) || connections.iter().any(|c| c.incomplete));
        Ok(BranchGraphView {
            snapshot_id: id,
            reference_target,
            targets,
            anchors,
            connections,
            has_more,
            loaded_commit_count: commits.len(),
        })
    }
    pub(super) fn history(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        query: CommitHistoryQuery,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<CommitHistoryPageView, String> {
        if query.target.repository_id() != repository.id.as_str() {
            return Err("The history repository changed.".into());
        }
        if !(1..=100).contains(&page_size) {
            return Err("Invalid history page size.".into());
        }
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&query).map_err(|error| error.to_string())?)
        );
        let offset = match cursor {
            None => 0,
            Some(cursor) => {
                let (scope, position) = cursor.split_once(':').ok_or("Invalid history cursor")?;
                if scope != fingerprint {
                    return Err("This cursor belongs to a different history range.".into());
                }
                position
                    .parse::<usize>()
                    .map_err(|_| "Invalid history position")?
            }
        };
        let object = |value: &str| ObjectId::parse(value).map_err(|error| error.to_string());
        let (facts, total_count) = match &query.scope {
            HistoryScope::Ancestry {
                tip_object_id,
                excluded_base_object_id,
            } => context
                .commits()
                .history_page(
                    repository.top_level.path(),
                    &object(tip_object_id)?,
                    excluded_base_object_id
                        .as_deref()
                        .map(object)
                        .transpose()?
                        .as_ref(),
                    offset,
                    page_size,
                )
                .map_err(|error| error.to_string())?,
            HistoryScope::GraphRange {
                snapshot_id,
                range_id,
            } => {
                let (members, total) = {
                    let snapshots = self
                        .snapshots
                        .lock()
                        .map_err(|_| "Graph snapshots unavailable")?;
                    let snapshot = snapshots
                        .iter()
                        .find(|s| &s.id == snapshot_id && s.repository_id == repository.id.as_str())
                        .ok_or(EXPIRED)?;
                    let range = snapshot
                        .ranges
                        .get(range_id)
                        .ok_or("This history range is unavailable; reopen the branch selector.")?;
                    if !range.sources.contains(&query.target) {
                        return Err(
                            "This source does not contain the requested history range.".into()
                        );
                    }
                    if offset > range.members.len() {
                        return Err("Invalid history position".into());
                    }
                    (
                        range.members[offset..(offset + page_size).min(range.members.len())]
                            .to_vec(),
                        range.members.len(),
                    )
                };
                (
                    context
                        .commits()
                        .facts_for_objects(repository.top_level.path(), &members)
                        .map_err(|e| e.to_string())?,
                    total,
                )
            }
        };
        let next_offset = offset + facts.len();
        Ok(CommitHistoryPageView {
            scope: query.scope,
            total_count,
            commits: facts.into_iter().map(commit_view).collect(),
            next_cursor: (next_offset < total_count)
                .then(|| format!("{fingerprint}:{next_offset}")),
        })
    }
}
