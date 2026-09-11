use super::{branch_history::HistoryScope, branch_presentation::BranchView, domain::ReviewTarget};
use crate::repository_context::CommitParents;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphAnchor {
    pub(crate) object_id: String,
    pub(crate) parent_ids: Vec<String>,
    pub(crate) boundary: bool,
    pub(crate) merge: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphConnection {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) commit_count: usize,
    pub(crate) collapsed: bool,
    pub(crate) incomplete: bool,
    pub(crate) eligible_sources: Vec<ReviewTarget>,
    pub(crate) scope: HistoryScope,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchGraphView {
    pub(crate) snapshot_id: String,
    pub(crate) reference_target: Option<ReviewTarget>,
    pub(crate) targets: Vec<BranchView>,
    pub(crate) anchors: Vec<GraphAnchor>,
    pub(crate) connections: Vec<GraphConnection>,
    pub(crate) has_more: bool,
    pub(crate) loaded_commit_count: usize,
}

pub(super) struct ProjectedRange {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) members: Vec<String>,
    pub(super) sources: Vec<usize>,
    pub(super) collapsed: bool,
    pub(super) incomplete: bool,
}
impl ProjectedRange {
    pub(super) fn id(&self) -> String {
        let mut hash = Sha256::new();
        for value in [&self.from, &self.to]
            .into_iter()
            .chain(self.members.iter())
        {
            hash.update(value.as_bytes());
            hash.update([0]);
        }
        format!("{:x}", hash.finalize())
    }
}

/// Keep current heads and their nearest common ancestors, not historical merge topology.
/// Input is topologically ordered, newest first.
pub(super) fn project(
    commits: &[CommitParents],
    tips: &[String],
) -> (Vec<GraphAnchor>, Vec<ProjectedRange>) {
    let mut ids: Vec<String> = commits
        .iter()
        .map(|c| c.object_id.as_str().into())
        .collect();
    let loaded = ids.len();
    let loaded_ids: HashSet<_> = ids.iter().cloned().collect();
    let missing: BTreeSet<_> = commits
        .iter()
        .flat_map(|c| c.parents.iter().map(|p| p.as_str()))
        .chain(tips.iter().map(String::as_str))
        .filter(|id| !loaded_ids.contains(*id))
        .map(str::to_owned)
        .collect();
    ids.extend(missing);
    let index: HashMap<_, _> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut parents = vec![Vec::new(); ids.len()];
    let mut children = parents.clone();
    for (i, commit) in commits.iter().enumerate() {
        for parent in &commit.parents {
            let j = index[parent.as_str()];
            parents[i].push(j);
            children[j].push(i);
        }
    }
    let mut sources = vec![BTreeSet::new(); ids.len()];
    let heads: BTreeSet<_> = tips.iter().map(|tip| index[tip.as_str()]).collect();
    for (source, tip) in tips.iter().enumerate() {
        sources[index[tip.as_str()]].insert(source);
    }
    for i in 0..loaded {
        let inherited = sources[i].clone();
        for &p in &parents[i] {
            sources[p].extend(&inherited);
        }
    }
    // Aliases of one head must not create an artificial branch point.
    let signatures: Vec<BTreeSet<usize>> = sources
        .iter()
        .map(|set| {
            set.iter()
                .map(|&source| index[tips[source].as_str()])
                .collect()
        })
        .collect();
    let mut retained = heads.clone();
    for i in 0..loaded {
        if signatures[i].len() < 2 || children[i].iter().any(|&c| signatures[c] == signatures[i]) {
            continue;
        }
        let signature: Vec<_> = signatures[i].iter().copied().collect();
        if signature.iter().enumerate().any(|(a, first)| {
            signature[a + 1..].iter().any(|second| {
                !children[i]
                    .iter()
                    .any(|&c| signatures[c].contains(first) && signatures[c].contains(second))
            })
        }) {
            retained.insert(i);
        }
    }
    let ancestors = |start: usize| {
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            if seen.insert(i) {
                stack.extend(&parents[i]);
            }
        }
        seen
    };
    let mut ancestry: HashMap<_, _> = retained.iter().map(|&i| (i, ancestors(i))).collect();
    if heads.len() > 1 {
        for i in loaded..ids.len() {
            // Hide an unloaded common tail only after all its current heads have converged.
            let common_tail = retained.iter().any(|&r| {
                r < loaded
                    && signatures[r].len() > 1
                    && signatures[r] == signatures[i]
                    && ancestry[&r].contains(&i)
            });
            if !common_tail {
                retained.insert(i);
                ancestry.insert(i, ancestors(i));
            }
        }
    }
    let mut ranges = Vec::new();
    for &to in &retained {
        if to >= loaded {
            continue;
        }
        let mut covered: HashSet<usize> = HashSet::new();
        for &from in retained.range((to + 1)..) {
            if !ancestry[&to].contains(&from) || covered.contains(&from) {
                continue;
            }
            covered.extend(&ancestry[&from]);
            let difference: HashSet<_> = ancestry[&to]
                .difference(&ancestry[&from])
                .copied()
                .collect();
            let members: Vec<_> = (0..loaded).filter(|i| difference.contains(i)).collect();
            ranges.push(ProjectedRange {
                from: ids[from].clone(),
                to: ids[to].clone(),
                collapsed: members.iter().any(|&i| parents[i].len() > 1),
                incomplete: from >= loaded || difference.iter().any(|&i| i >= loaded),
                members: members.into_iter().map(|i| ids[i].clone()).collect(),
                sources: sources[to].iter().copied().collect(),
            });
        }
    }
    let anchors = retained
        .into_iter()
        .map(|i| GraphAnchor {
            object_id: ids[i].clone(),
            parent_ids: ranges
                .iter()
                .filter(|e| e.to == ids[i])
                .map(|e| e.from.clone())
                .collect(),
            boundary: i >= loaded,
            merge: parents[i].len() > 1,
        })
        .collect();
    (anchors, ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_context::ObjectId;
    fn id(n: usize) -> String {
        format!("{n:040x}")
    }
    fn node(n: usize, parents: &[usize]) -> CommitParents {
        CommitParents {
            object_id: ObjectId::parse(id(n)).unwrap(),
            parents: parents
                .iter()
                .map(|&p| ObjectId::parse(id(p)).unwrap())
                .collect(),
        }
    }
    fn reachable(start: &str, end: &str, parents: &HashMap<String, Vec<String>>) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start.to_owned()];
        while let Some(id) = stack.pop() {
            if id == end {
                return true;
            }
            if visited.insert(id.clone()) {
                stack.extend(parents.get(&id).into_iter().flatten().cloned());
            }
        }
        false
    }
    fn assert_projection(
        commits: &[CommitParents],
        tips: &[String],
    ) -> (Vec<GraphAnchor>, Vec<ProjectedRange>) {
        let (anchors, ranges) = project(commits, tips);
        let before: HashMap<_, _> = commits
            .iter()
            .map(|c| {
                (
                    c.object_id.as_str().to_owned(),
                    c.parents.iter().map(|p| p.as_str().to_owned()).collect(),
                )
            })
            .collect();
        let after = anchors
            .iter()
            .map(|a| (a.object_id.clone(), a.parent_ids.clone()))
            .collect();
        for a in &anchors {
            for b in &anchors {
                assert_eq!(
                    reachable(&a.object_id, &b.object_id, &before),
                    reachable(&a.object_id, &b.object_id, &after),
                    "{} -> {}",
                    a.object_id,
                    b.object_id
                );
            }
        }
        for tip in tips {
            assert!(anchors.iter().any(|a| &a.object_id == tip));
        }
        for range in &ranges {
            assert_eq!(range.members.first(), Some(&range.to));
            assert!(!range.members.contains(&range.from));
            assert_eq!(
                range.members.len(),
                range.members.iter().collect::<HashSet<_>>().len()
            );
            for member in &range.members {
                assert!(reachable(&range.to, member, &before));
                assert!(!reachable(&range.from, member, &before));
            }
            let expected: Vec<_> = tips
                .iter()
                .enumerate()
                .filter(|(_, tip)| reachable(tip, &range.to, &before))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(range.sources, expected);
        }
        (anchors, ranges)
    }
    #[test]
    fn worktree_graph_folds_retired_nested_merges_but_preserves_surviving_targets() {
        let commits = vec![
            node(8, &[7, 6]),
            node(7, &[5]),
            node(6, &[5, 4]),
            node(5, &[3]),
            node(4, &[2]),
            node(3, &[2]),
            node(2, &[1]),
            node(1, &[]),
        ];
        let (anchors, ranges) = assert_projection(&commits, &[id(8)]);
        assert_eq!(anchors.len(), 1);
        assert!(ranges.is_empty());
        let (anchors, ranges) = assert_projection(&commits, &[id(8), id(2)]);
        assert_eq!(anchors.len(), 2);
        assert_eq!(ranges[0].members, (3..=8).rev().map(id).collect::<Vec<_>>());
        let (anchors, _) = assert_projection(&commits, &[id(8), id(4), id(4)]);
        assert!(anchors.iter().any(|a| a.object_id == id(4)));
    }
    #[test]
    fn worktree_graph_keeps_unrelated_roots_and_unloaded_targets_explicit() {
        let commits = vec![
            node(9, &[8, 7]),
            node(8, &[6]),
            node(7, &[6]),
            node(5, &[4]),
        ];
        let (anchors, _) = assert_projection(&commits, &[id(9), id(5), id(2)]);
        for n in [6, 4, 2] {
            assert!(anchors.iter().any(|a| a.object_id == id(n) && a.boundary));
        }
    }
    #[test]
    fn worktree_graph_reclaims_a_long_series_of_closed_merge_regions() {
        let mut commits = vec![node(1, &[])];
        let mut base = 1;
        for _ in 0..100 {
            commits.extend([
                node(base + 1, &[base]),
                node(base + 2, &[base]),
                node(base + 3, &[base + 1, base + 2]),
            ]);
            base += 3;
        }
        commits.reverse();
        let (anchors, ranges) = assert_projection(&commits, &[id(base), id(1)]);
        assert_eq!(anchors.len(), 2);
        assert_eq!(ranges[0].members.len(), 300);
    }
    #[test]
    fn worktree_graph_omits_the_shared_prefix_and_retired_diamonds() {
        let commits = vec![
            node(10, &[9]),
            node(9, &[7, 8]),
            node(8, &[6]),
            node(7, &[6]),
            node(6, &[4]),
            node(5, &[4]),
            node(4, &[3]),
            node(3, &[2, 1]),
            node(2, &[1]),
            node(1, &[]),
        ];
        let (anchors, ranges) = assert_projection(&commits, &[id(10), id(5)]);
        assert_eq!(
            anchors.iter().map(|a| &a.object_id).collect::<Vec<_>>(),
            vec![&id(10), &id(5), &id(4)]
        );
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].members, vec![id(10), id(9), id(8), id(7), id(6)]);
    }
    #[test]
    fn worktree_graph_retains_multiple_best_bases_in_criss_cross_history() {
        let commits = vec![
            node(5, &[3, 2]),
            node(4, &[2, 3]),
            node(3, &[1]),
            node(2, &[1]),
            node(1, &[]),
        ];
        let (anchors, ranges) = assert_projection(&commits, &[id(5), id(4)]);
        assert_eq!(anchors.len(), 4);
        assert_eq!(ranges.len(), 4);
        assert!(!anchors.iter().any(|a| a.object_id == id(1)));
    }
    #[test]
    fn worktree_graph_hides_an_unloaded_common_tail_but_marks_partial_side_history() {
        let commits = vec![node(7, &[5, 4]), node(6, &[5]), node(5, &[3])];
        let (anchors, ranges) = assert_projection(&commits, &[id(7), id(6)]);
        assert!(!anchors.iter().any(|a| a.object_id == id(3)));
        assert!(anchors.iter().any(|a| a.object_id == id(4) && a.boundary));
        assert!(ranges.iter().find(|r| r.to == id(7)).unwrap().incomplete);
        assert!(!ranges.iter().find(|r| r.to == id(6)).unwrap().incomplete);
    }
    #[test]
    fn worktree_graph_preserves_reachability_for_varied_parallel_paths() {
        // Deterministic DAGs exercise open regions that cannot safely collapse to one edge.
        for seed in 1..24usize {
            let commits: Vec<_> = (1..=30)
                .rev()
                .map(|n| {
                    let mut parents = if n > 1 { vec![n - 1] } else { vec![] };
                    if n > 3 && (n * seed) % 3 == 0 {
                        parents.push(1 + (n * seed) % (n - 2));
                    }
                    parents.sort_unstable();
                    parents.dedup();
                    node(n, &parents)
                })
                .collect();
            assert_projection(&commits, &[id(30), id(13), id(20)]);
        }
    }
}
