use super::*;
use rusqlite::{params, ErrorCode};
use std::{fs, path::Path, process::Command};

const TARGET_FIXTURE_SCHEMA: &str = r#"
CREATE TABLE repos (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL
);
CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    name TEXT NOT NULL
);
CREATE TABLE worktrees (
    id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    branch_id TEXT,
    path TEXT NOT NULL
);
"#;

#[test]
fn queries_only_branch_associated_worktrees_with_exact_nested_facts() {
    let fixture = TargetFixture::new();
    fixture.insert_repo("repo-a", "Orchestrator", "C:/Repos/Orchestrator");
    fixture.insert_repo("repo-b", "Other", "D:/Repos/Other");
    fixture.insert_branch("branch-a", "repo-a", "feature/workflows");
    fixture.insert_branch("branch-b", "repo-b", "main");
    fixture.insert_worktree(
        "worktree-a",
        "repo-a",
        Some("branch-a"),
        "C:/Worktrees/workflows",
    );
    fixture.insert_worktree("worktree-detached", "repo-a", None, "C:/Worktrees/detached");
    fixture.insert_worktree(
        "worktree-mismatched",
        "repo-a",
        Some("branch-b"),
        "C:/Worktrees/mismatched",
    );

    let targets = fixture.query_targets().expect("query targets");

    assert_eq!(
        targets,
        vec![ResolvedRepoBranchWorktreeTarget {
            repository: ResolvedRepositoryTarget {
                id: "repo-a".into(),
                name: "Orchestrator".into(),
                root_path: "C:/Repos/Orchestrator".into(),
            },
            branch: ResolvedBranchTarget {
                id: "branch-a".into(),
                name: "feature/workflows".into(),
            },
            worktree: ResolvedWorktreeTarget {
                id: "worktree-a".into(),
                path: "C:/Worktrees/workflows".into(),
            },
        }]
    );
    assert_eq!(
        serde_json::to_value(&targets[0]).expect("serialize target"),
        serde_json::json!({
            "repository": {
                "id": "repo-a",
                "name": "Orchestrator",
                "rootPath": "C:/Repos/Orchestrator"
            },
            "branch": { "id": "branch-a", "name": "feature/workflows" },
            "worktree": { "id": "worktree-a", "path": "C:/Worktrees/workflows" }
        })
    );
}

#[test]
fn query_sorts_targets_deterministically_across_repositories_branches_and_paths() {
    let fixture = TargetFixture::new();
    fixture.insert_repo("repo-z", "Zulu", "C:/Repos/Zulu");
    fixture.insert_repo("repo-a-2", "alpha", "D:/Repos/Alpha");
    fixture.insert_repo("repo-a-1", "Alpha", "C:/Repos/Alpha");
    fixture.insert_branch("branch-z", "repo-z", "main");
    fixture.insert_branch("branch-a-2", "repo-a-2", "zeta");
    fixture.insert_branch("branch-a-1-z", "repo-a-1", "Zeta");
    fixture.insert_branch("branch-a-1-a", "repo-a-1", "alpha");
    fixture.insert_worktree("wt-z", "repo-z", Some("branch-z"), "C:/Wt/Zulu");
    fixture.insert_worktree("wt-a-2", "repo-a-2", Some("branch-a-2"), "D:/Wt/Alpha");
    fixture.insert_worktree("wt-a-1-z", "repo-a-1", Some("branch-a-1-z"), "C:/Wt/Zeta");
    fixture.insert_worktree("wt-a-1-a", "repo-a-1", Some("branch-a-1-a"), "C:/Wt/Alpha");

    let first = fixture.query_targets().expect("first query");
    let second = fixture.query_targets().expect("second query");
    let ids = first
        .iter()
        .map(|target| target.worktree.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["wt-a-1-a", "wt-a-1-z", "wt-a-2", "wt-z"]);
    assert_eq!(second, first);
}

#[test]
fn source_connection_is_read_only() {
    let fixture = TargetFixture::new();
    fixture.insert_repo("repo", "Repo", "C:/Repo");
    let source = fixture.source();
    let connection = source
        .open_read_only_connection()
        .expect("open read-only source");

    let error = connection
        .execute(
            "INSERT INTO repos(id,name,root_path) VALUES('other','Other','C:/Other')",
            [],
        )
        .expect_err("read-only source must reject writes");

    match error {
        rusqlite::Error::SqliteFailure(code, _) => assert_eq!(code.code, ErrorCode::ReadOnly),
        other => panic!("unexpected read-only error: {other}"),
    }
}

#[test]
fn live_filter_requires_the_current_branch_and_discovers_each_repository_once() {
    let fixture = TargetFixture::new();
    fixture.insert_repo("repo", "Repository", "C:/Repository");
    fixture.insert_branch("branch-stale", "repo", "feature/stale");
    fixture.insert_branch("branch-current", "repo", "feature/current");
    fixture.insert_worktree(
        "worktree-stale",
        "repo",
        Some("branch-stale"),
        "C:/Worktrees/stale",
    );
    fixture.insert_worktree(
        "worktree-current",
        "repo",
        Some("branch-current"),
        "C:/Worktrees/current",
    );
    let candidates = fixture.query_targets().expect("query candidates");
    let mut discovery_count = 0;

    let targets = filter_current_worktree_targets(candidates, |repository_root| {
        discovery_count += 1;
        assert_eq!(repository_root, "C:/Repository");
        Ok(vec![
            CurrentBranchWorktree {
                path: "C:/Worktrees/stale".into(),
                branch_name: "feature/moved".into(),
            },
            CurrentBranchWorktree {
                path: "C:\\Worktrees\\current".into(),
                branch_name: "feature/current".into(),
            },
        ])
    })
    .expect("filter current targets");

    assert_eq!(discovery_count, 1);
    assert_eq!(
        targets
            .iter()
            .map(|target| target.worktree.id.as_str())
            .collect::<Vec<_>>(),
        vec!["worktree-current"]
    );
}

#[test]
fn source_omits_removed_worktrees_while_retaining_their_registry_rows() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }

    let fixture = TargetFixture::new();
    let repository = fixture.path("repository");
    let removed_worktree = fixture.path("removed-worktree");
    let missing_worktree = fixture.path("missing-worktree");
    fs::create_dir_all(&repository).expect("create repository directory");
    run_git(&repository, &["init", "-b", "main"]);
    run_git(&repository, &["config", "user.name", "Codex Test"]);
    run_git(
        &repository,
        &["config", "user.email", "codex-test@example.invalid"],
    );
    run_git(
        &repository,
        &["commit", "--allow-empty", "-m", "Initial commit"],
    );
    let removed_path = removed_worktree.to_string_lossy().into_owned();
    let missing_path = missing_worktree.to_string_lossy().into_owned();
    run_git(
        &repository,
        &["worktree", "add", "-b", "feature/removed", &removed_path],
    );
    run_git(
        &repository,
        &["worktree", "add", "-b", "feature/missing", &missing_path],
    );

    let repository_path = repository.to_string_lossy().into_owned();
    fixture.insert_repo("repo", "Repository", &repository_path);
    fixture.insert_branch("branch-main", "repo", "main");
    fixture.insert_branch("branch-removed", "repo", "feature/removed");
    fixture.insert_branch("branch-missing", "repo", "feature/missing");
    fixture.insert_worktree(
        "worktree-main",
        "repo",
        Some("branch-main"),
        &repository_path,
    );
    fixture.insert_worktree(
        "worktree-removed",
        "repo",
        Some("branch-removed"),
        &removed_path,
    );
    fixture.insert_worktree(
        "worktree-missing",
        "repo",
        Some("branch-missing"),
        &missing_path,
    );

    let before = fixture.source().list().expect("list current worktrees");
    assert_eq!(before.len(), 3);

    run_git(
        &repository,
        &["worktree", "remove", "--force", &removed_path],
    );
    fs::remove_dir_all(&missing_worktree).expect("remove worktree directory");

    assert_eq!(fixture.worktree_count(), 3);
    assert!(
        crate::git_worktree_facts(&repository_path)
            .expect("discover retained Git registrations")
            .iter()
            .any(|worktree| crate::same_filesystem_path(&worktree.path, &missing_path)),
        "Git should still report the missing worktree as prunable"
    );

    let after = fixture.source().list().expect("list remaining worktrees");
    assert_eq!(
        after
            .iter()
            .map(|target| target.worktree.id.as_str())
            .collect::<Vec<_>>(),
        vec!["worktree-main"]
    );
}

struct TargetFixture {
    _directory: tempfile::TempDir,
    database_path: PathBuf,
}

impl TargetFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary target database");
        let database_path = directory.path().join("codex-orchestrator.sqlite");
        let connection = Connection::open(&database_path).expect("open target fixture");
        connection
            .execute_batch(TARGET_FIXTURE_SCHEMA)
            .expect("initialize target fixture");
        drop(connection);
        Self {
            _directory: directory,
            database_path,
        }
    }

    fn source(&self) -> DiscoveredWorktreeTargetSource {
        DiscoveredWorktreeTargetSource::new(self.database_path.clone())
    }

    fn path(&self, name: &str) -> PathBuf {
        self._directory.path().join(name)
    }

    fn query_targets(&self) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
        let connection = Connection::open(&self.database_path).expect("open target fixture");
        query_discovered_worktree_targets(&connection)
    }

    fn worktree_count(&self) -> i64 {
        let connection = Connection::open(&self.database_path).expect("open target fixture");
        connection
            .query_row("SELECT COUNT(*) FROM worktrees", [], |row| row.get(0))
            .expect("count worktrees")
    }

    fn insert_repo(&self, id: &str, name: &str, root_path: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO repos(id,name,root_path) VALUES(?1,?2,?3)",
                    params![id, name, root_path],
                )
                .expect("insert repository");
        });
    }

    fn insert_branch(&self, id: &str, repo_id: &str, name: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO branches(id,repo_id,name) VALUES(?1,?2,?3)",
                    params![id, repo_id, name],
                )
                .expect("insert branch");
        });
    }

    fn insert_worktree(&self, id: &str, repo_id: &str, branch_id: Option<&str>, path: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO worktrees(id,repo_id,branch_id,path) VALUES(?1,?2,?3,?4)",
                    params![id, repo_id, branch_id, path],
                )
                .expect("insert worktree");
        });
    }

    fn with_connection(&self, operation: impl FnOnce(&Connection)) {
        let connection = Connection::open(&self.database_path).expect("open target fixture");
        operation(&connection);
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .expect("run git command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}
