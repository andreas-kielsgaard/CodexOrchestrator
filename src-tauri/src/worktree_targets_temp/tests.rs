use super::*;
use rusqlite::{params, ErrorCode};

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
fn lists_only_branch_associated_worktrees_with_exact_nested_facts() {
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

    let targets = fixture.source().list().expect("list targets");

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
fn sorts_targets_deterministically_across_repositories_branches_and_paths() {
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

    let first = fixture.source().list().expect("first list");
    let second = fixture.source().list().expect("second list");
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
