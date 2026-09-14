use super::{
    branch_history::{BranchHistoryService, CommitHistoryQuery, HistoryScope},
    branch_inventory::inventory,
    build_presentation::{BuildWorkspacePlanInput, CreateBuildSourceInput},
    domain::*,
    source_materialization::SourceMaterializationService,
    storage::{ReviewBuildRepository, WorkspaceRepository, WorktreeReviewDatabase},
    worktree_activity::WorktreeActivityService,
};
use crate::repository_context::{RepositoryContext, RepositoryIdentity};
use chrono::Utc;
use std::{fs, path::Path, process::Command, sync::Arc};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

struct Fixture {
    temp: tempfile::TempDir,
    root: std::path::PathBuf,
    context: RepositoryContext,
    repository: RepositoryIdentity,
    base: String,
    side: String,
    tip: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repository");
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["config", "user.name", "Review Test"]);
        git(&root, &["config", "user.email", "review@example.invalid"]);
        fs::write(root.join("shared.txt"), "base").unwrap();
        fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Base"]);
        let base = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["checkout", "-b", "side"]);
        fs::write(root.join("side.txt"), "side").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Side"]);
        let side = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["checkout", "main"]);
        fs::write(root.join("main.txt"), "main").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Main"]);
        git(&root, &["merge", "--no-ff", "side", "-m", "Merge side"]);
        let tip = git(&root, &["rev-parse", "HEAD"]);
        let context = RepositoryContext::discover().unwrap();
        let repository = context.identities().inspect(&root).unwrap();
        Self {
            temp,
            root,
            context,
            repository,
            base,
            side,
            tip,
        }
    }
    fn target(&self) -> ReviewTarget {
        ReviewTarget::Branch {
            repository_id: self.repository.id.as_str().into(),
            branch_ref: "refs/heads/main".into(),
        }
    }
}

#[test]
fn worktree_history_paging_includes_merged_commits_and_keeps_pinned_scope_after_ref_moves() {
    let fixture = Fixture::new();
    let service = BranchHistoryService::default();
    let query = CommitHistoryQuery {
        target: fixture.target(),
        scope: HistoryScope::Ancestry {
            tip_object_id: fixture.tip.clone(),
            excluded_base_object_id: Some(fixture.base.clone()),
        },
    };
    let mut page = service
        .history(
            &fixture.context,
            &fixture.repository,
            query.clone(),
            None,
            1,
        )
        .unwrap();
    assert_eq!(page.total_count, 3);
    let cursor = page.next_cursor.clone().unwrap();
    git(
        &fixture.root,
        &["commit", "--allow-empty", "-m", "Moved tip"],
    );
    let mut seen = Vec::new();
    loop {
        seen.extend(page.commits.iter().map(|commit| commit.object_id.clone()));
        let Some(cursor) = page.next_cursor else {
            break;
        };
        page = service
            .history(
                &fixture.context,
                &fixture.repository,
                query.clone(),
                Some(&cursor),
                1,
            )
            .unwrap();
    }
    assert_eq!(seen.len(), 3);
    assert_eq!(
        seen.iter().collect::<std::collections::HashSet<_>>().len(),
        3
    );
    assert!(seen.contains(&fixture.side));
    assert!(!seen.contains(&fixture.base));
    let other = CommitHistoryQuery {
        target: fixture.target(),
        scope: HistoryScope::Ancestry {
            tip_object_id: fixture.side.clone(),
            excluded_base_object_id: None,
        },
    };
    assert!(service
        .history(
            &fixture.context,
            &fixture.repository,
            other,
            Some(&cursor),
            1
        )
        .is_err());
    // Both merged paths compress to the same anchors once the side ref is removed.
    git(&fixture.root, &["branch", "-d", "side"]);
    git(&fixture.root, &["branch", "z-base", &fixture.base]);
    let database = WorktreeReviewDatabase::open_in_memory().unwrap();
    let activity = WorktreeActivityService::default();
    let targets = inventory(&fixture.context, &fixture.repository, &database, &activity).unwrap();
    let graph = service
        .graph(&fixture.context, &fixture.repository, 1200, None, || {
            Ok(targets)
        })
        .unwrap();
    assert_eq!(graph.anchors.len(), 2);
    assert_eq!(graph.connections.len(), 1);
    assert!(graph.connections[0].collapsed);
    assert_eq!(graph.connections[0].commit_count, 4);
    for edge in graph.connections {
        let page = service
            .history(
                &fixture.context,
                &fixture.repository,
                CommitHistoryQuery {
                    target: fixture.target(),
                    scope: edge.scope,
                },
                None,
                100,
            )
            .unwrap();
        let expected = git(
            &fixture.root,
            &["rev-list", &edge.to, &format!("^{}", edge.from)],
        );
        let expected: std::collections::HashSet<_> = expected.lines().collect();
        let actual: std::collections::HashSet<_> =
            page.commits.iter().map(|c| c.object_id.as_str()).collect();
        assert_eq!(actual, expected);
        assert!(!edge.incomplete);
        assert_eq!(page.total_count, edge.commit_count);
        assert_eq!(page.commits.len(), edge.commit_count);
        assert!(!page
            .commits
            .iter()
            .any(|commit| commit.object_id == edge.from));
    }
}

#[test]
fn worktree_inventory_retains_detached_checkouts_after_branch_deletion_and_discovers_only_source_edits(
) {
    let fixture = Fixture::new();
    let detached = fixture.temp.path().join("detached");
    git(
        &fixture.root,
        &[
            "worktree",
            "add",
            "--detach",
            detached.to_str().unwrap(),
            &fixture.side,
        ],
    );
    git(&fixture.root, &["branch", "-D", "side"]);
    let database = WorktreeReviewDatabase::open_in_memory().unwrap();
    let activity = WorktreeActivityService::default();
    let targets = inventory(&fixture.context, &fixture.repository, &database, &activity).unwrap();
    assert_eq!(targets.len(), 2);
    let detached_target = targets
        .iter()
        .find(|target| matches!(target.target, ReviewTarget::Worktree { .. }))
        .unwrap();
    assert_eq!(detached_target.branch_ref, None);
    assert_eq!(detached_target.available_worktree_count, 1);
    fs::create_dir(detached.join("node_modules")).unwrap();
    fs::write(detached.join("node_modules/ignored.js"), "ignored").unwrap();
    fs::write(detached.join("source.txt"), "new source").unwrap();
    let estimates = activity
        .discover(
            &fixture.context,
            &fixture.repository,
            &detached_target.worktree_ids,
        )
        .unwrap();
    assert_eq!(estimates.len(), 1);
    assert_eq!(estimates[0].estimate.untracked_files, 1);
    assert!(estimates[0].estimate.basis.contains("changed_file"));
    let cached = inventory(&fixture.context, &fixture.repository, &database, &activity).unwrap();
    assert!(cached
        .iter()
        .find(|target| target.target == detached_target.target)
        .unwrap()
        .activity
        .is_some());
}

#[test]
fn worktree_exact_commit_materializes_and_persists_without_a_surviving_branch() {
    let fixture = Fixture::new();
    let detached = fixture.temp.path().join("detached");
    git(
        &fixture.root,
        &[
            "worktree",
            "add",
            "--detach",
            detached.to_str().unwrap(),
            &fixture.side,
        ],
    );
    git(&fixture.root, &["branch", "-D", "side"]);
    let database = Arc::new(WorktreeReviewDatabase::open_in_memory().unwrap());
    let targets = inventory(
        &fixture.context,
        &fixture.repository,
        &database,
        &WorktreeActivityService::default(),
    )
    .unwrap();
    let target = targets
        .iter()
        .find(|target| matches!(target.target, ReviewTarget::Worktree { .. }))
        .unwrap();
    let source_context = CommitSourceContext::Worktree {
        worktree_id: target.worktree_ids[0].clone(),
        tip_object_id: fixture.side.clone(),
    };
    let service =
        SourceMaterializationService::new(database.clone(), fixture.temp.path().join("review"));
    let build_id = ReviewBuildId::random();
    let prepared = service
        .prepare(
            &fixture.context,
            &fixture.repository,
            None,
            &build_id,
            WorkspaceId::random(),
            &CreateBuildSourceInput::ExactCommit {
                object_id: fixture.base.clone(),
                context: source_context,
            },
            &BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                originating_association_id: None,
            },
        )
        .unwrap();
    assert!(
        !Path::new(prepared.workspace().location.as_str()).exists(),
        "planning must not instantiate a checkout"
    );
    let now = Utc::now();
    let build = ReviewBuild {
        id: build_id.clone(),
        name: ReviewBuildName::new("Detached historical build").unwrap(),
        source: prepared.source().clone(),
        workspace_id: prepared.workspace().id.clone(),
        retention_key: RetentionKey::new("detached-history").unwrap(),
        current_output_id: None,
        lifecycle: BuildLifecycle::Active,
        created_at: now,
        updated_at: now,
    };
    database
        .transaction(|transaction| {
            transaction.workspaces().save(prepared.workspace())?;
            transaction.builds().save(&build)
        })
        .unwrap();
    let materialized = service
        .materialize(&fixture.context, &fixture.repository, None, prepared)
        .unwrap();
    assert_eq!(
        git(
            Path::new(materialized.workspace().location.as_str()),
            &["rev-parse", "HEAD"]
        ),
        fixture.base
    );
    assert!(materialized.association().is_none());
    assert_eq!(
        database
            .builds()
            .find(&build_id)
            .unwrap()
            .unwrap()
            .source
            .branch_ref,
        None
    );
    assert!(matches!(
        materialized.workspace().ownership,
        WorkspaceOwnership::OwnedBuildWorktree { .. }
    ));
    // Direct physical source uses exactly the requested checkout and rejects a stale HEAD.
    let direct = CreateBuildSourceInput::PhysicalWorktree {
        worktree_id: target.worktree_ids[0].clone(),
        head_object_id: fixture.side.clone(),
        snapshot: false,
    };
    let plan = BuildWorkspacePlanInput::BorrowPhysicalWorktree {
        worktree_id: target.worktree_ids[0].clone(),
    };
    let accepted = service
        .prepare(
            &fixture.context,
            &fixture.repository,
            None,
            &ReviewBuildId::random(),
            WorkspaceId::random(),
            &direct,
            &plan,
        )
        .unwrap();
    assert!(matches!(
        accepted.workspace().ownership,
        WorkspaceOwnership::BorrowedPhysicalWorktree
    ));
    git(&detached, &["checkout", "--detach", &fixture.base]);
    assert!(service
        .prepare(
            &fixture.context,
            &fixture.repository,
            None,
            &ReviewBuildId::random(),
            WorkspaceId::random(),
            &direct,
            &plan
        )
        .is_err());
}

#[test]
fn worktree_graph_ranges_remain_exact_after_expansion_and_reject_expired_or_foreign_sources() {
    let fixture = Fixture::new();
    git(&fixture.root, &["branch", "-d", "side"]);
    git(&fixture.root, &["branch", "z-base", &fixture.base]);
    let database = WorktreeReviewDatabase::open_in_memory().unwrap();
    let activity = WorktreeActivityService::default();
    let targets = inventory(&fixture.context, &fixture.repository, &database, &activity).unwrap();
    let service = BranchHistoryService::default();
    let graph = service
        .graph(&fixture.context, &fixture.repository, 2, None, || {
            Ok(targets.clone())
        })
        .unwrap();
    assert!(graph.has_more);
    let edge = &graph.connections[0];
    let query = CommitHistoryQuery {
        target: fixture.target(),
        scope: edge.scope.clone(),
    };
    let before = service
        .history(
            &fixture.context,
            &fixture.repository,
            query.clone(),
            None,
            100,
        )
        .unwrap();
    git(
        &fixture.root,
        &["commit", "--allow-empty", "-m", "Moved after graph"],
    );
    let expanded = service
        .graph(
            &fixture.context,
            &fixture.repository,
            1200,
            Some(&graph.snapshot_id),
            || panic!("Expansion must retain pinned targets"),
        )
        .unwrap();
    assert_eq!(
        expanded
            .targets
            .iter()
            .find(|t| t.target == fixture.target())
            .unwrap()
            .tip
            .object_id,
        fixture.tip
    );
    assert_eq!(expanded.connections.len(), 1);
    let collapsed = &expanded.connections[0];
    let mut all = Vec::new();
    let mut cursor = None;
    loop {
        let page = service
            .history(
                &fixture.context,
                &fixture.repository,
                CommitHistoryQuery {
                    target: fixture.target(),
                    scope: collapsed.scope.clone(),
                },
                cursor.as_deref(),
                1,
            )
            .unwrap();
        all.extend(page.commits.into_iter().map(|c| c.object_id));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(all.len(), collapsed.commit_count);
    assert_eq!(
        all.iter().collect::<std::collections::HashSet<_>>().len(),
        all.len()
    );
    assert!(all.contains(&fixture.side));
    assert!(!all.contains(&fixture.base));
    let after = service
        .history(
            &fixture.context,
            &fixture.repository,
            query.clone(),
            None,
            100,
        )
        .unwrap();
    assert_eq!(
        before
            .commits
            .iter()
            .map(|c| &c.object_id)
            .collect::<Vec<_>>(),
        after
            .commits
            .iter()
            .map(|c| &c.object_id)
            .collect::<Vec<_>>()
    );
    let mut foreign = query.clone();
    foreign.target = ReviewTarget::Branch {
        repository_id: fixture.repository.id.as_str().into(),
        branch_ref: "refs/heads/side".into(),
    };
    assert!(service
        .history(&fixture.context, &fixture.repository, foreign, None, 100)
        .unwrap_err()
        .contains("source"));
    for _ in 0..8 {
        service
            .graph(&fixture.context, &fixture.repository, 1200, None, || {
                Ok(targets.clone())
            })
            .unwrap();
    }
    assert!(service
        .history(&fixture.context, &fixture.repository, query, None, 100)
        .unwrap_err()
        .contains("expired"));
}
