use super::*;
use rusqlite::{params, ErrorCode};
use std::{fs, path::Path, process::Command};

const TARGET_FIXTURE_SCHEMA: &str = r#"
CREATE TABLE repos (id TEXT PRIMARY KEY, name TEXT NOT NULL, root_path TEXT NOT NULL);
CREATE TABLE branches (id TEXT PRIMARY KEY, repo_id TEXT NOT NULL, name TEXT NOT NULL);
CREATE TABLE worktrees (id TEXT PRIMARY KEY, repo_id TEXT NOT NULL, branch_id TEXT, path TEXT NOT NULL);
"#;

#[test]
fn current_git_worktrees_expand_beyond_registry_rows_with_stable_identity() {
    let repositories = vec![RegisteredRepository {
        id: "repo".into(),
        name: "Orchestrator".into(),
        root_path: "C:/Repo".into(),
    }];
    let branches = vec![RegisteredBranch {
        id: "branch-main".into(),
        repository_id: "repo".into(),
        name: "main".into(),
    }];
    let worktrees = vec![RegisteredWorktree {
        id: "worktree-main".into(),
        repository_id: "repo".into(),
        branch_name: Some("main".into()),
        path: "C:/Repo".into(),
    }];

    let first = resolve_current_targets(
        repositories.clone(),
        branches.clone(),
        worktrees.clone(),
        |_| {
            Ok(vec![
                CurrentBranchWorktree {
                    path: "C:/Repo".into(),
                    branch_name: "main".into(),
                },
                CurrentBranchWorktree {
                    path: "C:/Worktrees/feature".into(),
                    branch_name: "feature/workflows".into(),
                },
            ])
        },
        |_| Ok("C:/Repo/.git".into()),
    )
    .unwrap();
    let second = resolve_current_targets(
        repositories,
        branches,
        worktrees,
        |_| {
            Ok(vec![
                CurrentBranchWorktree {
                    path: "C:/Repo".into(),
                    branch_name: "main".into(),
                },
                CurrentBranchWorktree {
                    path: "C:/Worktrees/feature".into(),
                    branch_name: "feature/workflows".into(),
                },
            ])
        },
        |_| Ok("C:/Repo/.git".into()),
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.len(), 2);
    let main = first
        .iter()
        .find(|target| target.branch.name == "main")
        .unwrap();
    assert_eq!(main.branch.id, "branch-main");
    assert_eq!(main.worktree.id, "worktree-main");
    let feature = first
        .iter()
        .find(|target| target.branch.name == "feature/workflows")
        .unwrap();
    assert!(feature.branch.id.starts_with("temporary-branch-"));
    assert!(feature.worktree.id.starts_with("temporary-worktree-"));
    assert_eq!(feature.repository.git_common_directory, "C:/Repo/.git");
    assert_eq!(
        serde_json::to_value(feature).unwrap(),
        serde_json::json!({
            "repository": {
                "id": "repo",
                "name": "Orchestrator",
                "gitCommonDirectory": "C:/Repo/.git"
            },
            "branch": {
                "id": feature.branch.id,
                "name": "feature/workflows"
            },
            "worktree": {
                "id": feature.worktree.id,
                "path": "C:/Worktrees/feature"
            }
        })
    );
}

#[test]
fn source_connection_is_read_only() {
    let fixture = TargetFixture::new();
    fixture.insert_repo("repo", "Repo", "C:/Repo");
    let connection = fixture
        .source()
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
fn source_uses_current_branch_attached_git_worktrees_and_omits_removed_or_detached() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let fixture = TargetFixture::new();
    let repository = fixture.path("repository");
    let feature = fixture.path("feature-worktree");
    let detached = fixture.path("detached-worktree");
    fs::create_dir_all(&repository).unwrap();
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
    run_git(
        &repository,
        &[
            "worktree",
            "add",
            "-b",
            "feature/workflows",
            &feature.to_string_lossy(),
        ],
    );
    run_git(
        &repository,
        &["worktree", "add", "--detach", &detached.to_string_lossy()],
    );
    let repository_path = repository.to_string_lossy();
    fixture.insert_repo("repo", "Repository", &repository_path);
    fixture.insert_branch("branch-main", "repo", "main");
    fixture.insert_worktree(
        "worktree-main",
        "repo",
        Some("branch-main"),
        &repository_path,
    );

    let before = fixture.source().list().unwrap();
    assert_eq!(
        before
            .iter()
            .map(|target| target.branch.name.as_str())
            .collect::<Vec<_>>(),
        vec!["feature/workflows", "main"]
    );
    assert!(before
        .iter()
        .all(|target| target.repository.git_common_directory.ends_with(".git")));

    run_git(
        &repository,
        &["worktree", "remove", "--force", &feature.to_string_lossy()],
    );
    let after = fixture.source().list().unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].branch.name, "main");
    assert_eq!(
        fixture.worktree_count(),
        1,
        "temporary discovery must not write"
    );
}

struct TargetFixture {
    directory: tempfile::TempDir,
    database_path: PathBuf,
}

impl TargetFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("codex-orchestrator.sqlite");
        let connection = Connection::open(&database_path).unwrap();
        connection.execute_batch(TARGET_FIXTURE_SCHEMA).unwrap();
        drop(connection);
        Self {
            directory,
            database_path,
        }
    }

    fn source(&self) -> DiscoveredWorktreeTargetSource {
        DiscoveredWorktreeTargetSource::new(self.database_path.clone())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn insert_repo(&self, id: &str, name: &str, root_path: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO repos(id,name,root_path) VALUES(?1,?2,?3)",
                    params![id, name, root_path],
                )
                .unwrap();
        });
    }

    fn insert_branch(&self, id: &str, repo_id: &str, name: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO branches(id,repo_id,name) VALUES(?1,?2,?3)",
                    params![id, repo_id, name],
                )
                .unwrap();
        });
    }

    fn insert_worktree(&self, id: &str, repo_id: &str, branch_id: Option<&str>, path: &str) {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO worktrees(id,repo_id,branch_id,path) VALUES(?1,?2,?3,?4)",
                    params![id, repo_id, branch_id, path],
                )
                .unwrap();
        });
    }

    fn worktree_count(&self) -> i64 {
        let connection = Connection::open(&self.database_path).unwrap();
        connection
            .query_row("SELECT COUNT(*) FROM worktrees", [], |row| row.get(0))
            .unwrap()
    }

    fn with_connection(&self, operation: impl FnOnce(&Connection)) {
        operation(&Connection::open(&self.database_path).unwrap());
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}
