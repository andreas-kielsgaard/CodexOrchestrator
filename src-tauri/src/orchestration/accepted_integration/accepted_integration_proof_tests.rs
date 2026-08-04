use super::*;
use rusqlite::params;
use std::process::Command;
use tempfile::TempDir;

fn run(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "NUL")
            .status()
            .unwrap()
            .success(),
        "git {args:?}"
    );
}

fn fixture() -> (TempDir, Connection, Row) {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    run(root, &["init", "-b", "main"]);
    run(root, &["config", "user.name", "Test User"]);
    run(root, &["config", "user.email", "test@example.invalid"]);
    fs::write(root.join("base.txt"), "base\n").unwrap();
    run(root, &["add", "."]);
    run(root, &["commit", "-m", "base"]);
    let baseline = git(root, &["rev-parse", "HEAD"]).unwrap();
    run(root, &["checkout", "-b", "candidate"]);
    fs::write(root.join("base.txt"), "candidate\n").unwrap();
    fs::write(root.join("candidate.txt"), "candidate\n").unwrap();
    run(root, &["add", "."]);
    run(root, &["commit", "-m", "candidate"]);
    let candidate = git(root, &["rev-parse", "HEAD"]).unwrap();
    let tree = git(root, &["rev-parse", "HEAD^{tree}"]).unwrap();
    run(
        root,
        &[
            "update-ref",
            "refs/codex/orchestrator/accepted/candidate",
            &candidate,
        ],
    );
    run(root, &["checkout", "main"]);

    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY);
             CREATE TABLE initiated_sprint_git_authorities(
               authority_id TEXT PRIMARY KEY,repository_root TEXT,repository_common_dir TEXT,
               worktree_root TEXT,baseline_object_id TEXT
             );
             CREATE TABLE accepted_handler_candidates(
               candidate_id TEXT PRIMARY KEY,work_unit_id TEXT,authority_id TEXT,pinned_at TEXT,
               attention_reason TEXT,attempt_baseline_object_id TEXT,candidate_commit_id TEXT,
               candidate_tree_id TEXT,private_ref_name TEXT,evidence_fingerprint TEXT
             );
             CREATE TABLE sprint_target_currents(
               authority_id TEXT PRIMARY KEY,target_ref_name TEXT,current_object_id TEXT,
               binding_fingerprint TEXT,version INTEGER,attention_reason TEXT,updated_at TEXT
             );
             CREATE TABLE work_unit_relationships(
               relationship_id TEXT PRIMARY KEY,relationship_kind TEXT,from_id TEXT,to_id TEXT
             );
             INSERT INTO work_units VALUES('unit');",
        )
        .unwrap();
    let common = root.join(".git");
    connection
        .execute(
            "INSERT INTO initiated_sprint_git_authorities VALUES('authority',?1,?2,?1,?3)",
            params![root.to_string_lossy(), common.to_string_lossy(), baseline],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO accepted_handler_candidates VALUES(
               'candidate','unit','authority',?1,NULL,?2,?3,?4,
               'refs/codex/orchestrator/accepted/candidate','evidence'
             )",
            params![now(), baseline, candidate, tree],
        )
        .unwrap();
    let binding = sprint_target_binding_fingerprint("authority", "refs/heads/main", &baseline);
    connection
        .execute(
            "INSERT INTO sprint_target_currents VALUES(
               'authority','refs/heads/main',?1,?2,1,NULL,?3
             )",
            params![baseline, binding, now()],
        )
        .unwrap();
    let row = Row {
        candidate: "candidate".into(),
        unit: "unit".into(),
        authority: "authority".into(),
        repo: root.to_path_buf(),
        common,
        worktree: root.to_path_buf(),
        baseline,
        commit: candidate,
        tree,
        private_ref: "refs/codex/orchestrator/accepted/candidate".into(),
        evidence: "evidence".into(),
    };
    (directory, connection, row)
}

#[derive(Debug, Eq, PartialEq)]
struct RuntimeState {
    symbolic_head: Option<String>,
    head: String,
    target_ref: String,
    index_tree: String,
    status: String,
    unstaged: String,
    staged: String,
}

fn runtime_state(row: &Row) -> RuntimeState {
    RuntimeState {
        symbolic_head: git(&row.worktree, &["symbolic-ref", "--quiet", "HEAD"]).ok(),
        head: git(&row.worktree, &["rev-parse", "HEAD^{commit}"]).unwrap(),
        target_ref: git(
            &row.worktree,
            &["show-ref", "--verify", "--hash", "refs/heads/main"],
        )
        .unwrap(),
        index_tree: git(&row.worktree, &["write-tree"]).unwrap(),
        status: git(&row.worktree, &["status", "--porcelain"]).unwrap(),
        unstaged: git(&row.worktree, &["diff", "--binary"]).unwrap(),
        staged: git(&row.worktree, &["diff", "--cached", "--binary"]).unwrap(),
    }
}

fn reserve_only(connection: &mut Connection, row: &Row) -> Integration {
    initialize_accepted_integration_schema(connection).unwrap();
    reserve(connection, row).unwrap();
    integration(connection, &row.candidate).unwrap().unwrap()
}

fn store_object(
    connection: &Connection,
    integration: &Integration,
    commit: &str,
    tree: &str,
    stored_fingerprint: &str,
) {
    connection
        .execute(
            "UPDATE accepted_work_unit_integrations
             SET integration_commit_id=?2,integration_tree_id=?3,commit_fingerprint=?4,
                 object_created_at=?5,stage='object_created'
             WHERE integration_id=?1",
            params![integration.id, commit, tree, stored_fingerprint, now()],
        )
        .unwrap();
}

fn assert_no_completion(connection: &Connection) {
    assert_eq!(
        connection
            .query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                row.get(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        0
    );
}

fn assert_attention(connection: &Connection) {
    assert_eq!(
        connection
            .query_row::<String, _, _>(
                "SELECT stage FROM accepted_work_unit_integrations",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        "attention"
    );
}

fn add_dependency(connection: &Connection) {
    connection
        .execute_batch(
            "INSERT INTO work_units VALUES('dependent');
             INSERT INTO work_unit_relationships VALUES('edge','depends_on','dependent','unit');",
        )
        .unwrap();
}

fn settled_fixture() -> (TempDir, Connection, Row) {
    let (directory, mut connection, row) = fixture();
    add_dependency(&connection);
    reconcile_accepted_integrations(&mut connection).unwrap();
    assert_eq!(
        git(&row.repo, &["rev-list", "--count", "HEAD"]).unwrap(),
        "2"
    );
    (directory, connection, row)
}

fn rewind_to_db_advanced(connection: &Connection) {
    connection
        .execute_batch(
            "UPDATE accepted_work_unit_integrations
             SET stage='db_advanced',settled_at=NULL,notification_intent_recorded_at=NULL;",
        )
        .unwrap();
}

#[test]
fn accepted_integration_target_ref_cas_loser_and_foreign_ref_are_not_overwritten() {
    let (_directory, mut connection, row) = fixture();
    let integration = reserve_only(&mut connection, &row);
    let tree = merged_tree(&row, &integration.pre).unwrap();
    let (created, fingerprint) = create_commit(&row, &integration, &tree).unwrap();
    store_object(&connection, &integration, &created, &tree, &fingerprint);
    run(
        &row.repo,
        &["update-ref", "refs/heads/main", &row.commit, &row.baseline],
    );
    assert!(git(
        &row.repo,
        &["update-ref", "refs/heads/main", &created, &row.baseline,],
    )
    .is_err());
    let before = runtime_state(&row);

    reconcile_accepted_integrations(&mut connection).unwrap();

    assert_eq!(runtime_state(&row), before);
    assert_attention(&connection);
    assert_no_completion(&connection);
}

#[test]
fn accepted_integration_dirty_worktree_and_foreign_index_fail_before_effect() {
    for staged in [false, true] {
        let (_directory, mut connection, row) = fixture();
        reserve_only(&mut connection, &row);
        fs::write(
            row.repo.join("foreign.txt"),
            if staged { "staged\n" } else { "dirty\n" },
        )
        .unwrap();
        if staged {
            run(&row.repo, &["add", "foreign.txt"]);
        }
        let before = runtime_state(&row);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before, "staged={staged}");
        assert_attention(&connection);
        assert_no_completion(&connection);
    }
}

#[test]
fn accepted_integration_wrong_common_dir_detached_and_wrong_branch_fail_closed() {
    {
        let (_directory, mut connection, row) = fixture();
        reserve_only(&mut connection, &row);
        let foreign = TempDir::new().unwrap();
        connection
            .execute(
                "UPDATE initiated_sprint_git_authorities SET repository_common_dir=?1",
                [foreign.path().to_string_lossy().as_ref()],
            )
            .unwrap();
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before);
        assert_attention(&connection);
        assert_no_completion(&connection);
    }
    for detached in [true, false] {
        let (_directory, mut connection, row) = fixture();
        reserve_only(&mut connection, &row);
        if detached {
            run(&row.repo, &["checkout", "--detach", &row.baseline]);
        } else {
            run(&row.repo, &["checkout", "candidate"]);
        }
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before, "detached={detached}");
        assert_attention(&connection);
        assert_no_completion(&connection);
    }
}

#[test]
fn accepted_integration_candidate_and_intent_correlations_reject_tamper() {
    let mutations = [
        "UPDATE accepted_handler_candidates SET candidate_tree_id=(SELECT pre_object_id FROM accepted_work_unit_integrations)",
        "UPDATE accepted_handler_candidates SET attempt_baseline_object_id=candidate_commit_id",
        "UPDATE accepted_handler_candidates SET evidence_fingerprint='forged-evidence'",
        "UPDATE accepted_work_unit_integrations SET target_ref_name='refs/heads/foreign'",
    ];
    for mutation in mutations {
        let (_directory, mut connection, row) = fixture();
        reserve_only(&mut connection, &row);
        connection.execute_batch(mutation).unwrap();
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before, "mutation={mutation}");
        assert_attention(&connection);
        assert_no_completion(&connection);
    }

    let (_directory, mut connection, row) = fixture();
    reserve_only(&mut connection, &row);
    run(
        &row.repo,
        &["update-ref", &row.private_ref, &row.baseline, &row.commit],
    );
    let before = runtime_state(&row);
    reconcile_accepted_integrations(&mut connection).unwrap();
    assert_eq!(runtime_state(&row), before);
    assert_attention(&connection);
    assert_no_completion(&connection);
}

#[test]
fn accepted_integration_three_tree_content_conflict_has_no_target_effect() {
    let (_directory, mut connection, row) = fixture();
    fs::write(row.repo.join("base.txt"), "target\n").unwrap();
    run(&row.repo, &["add", "base.txt"]);
    run(&row.repo, &["commit", "-m", "target"]);
    let target = git(&row.repo, &["rev-parse", "HEAD"]).unwrap();
    let binding = sprint_target_binding_fingerprint("authority", "refs/heads/main", &target);
    connection
        .execute(
            "UPDATE sprint_target_currents
             SET current_object_id=?1,binding_fingerprint=?2,version=2",
            params![target, binding],
        )
        .unwrap();
    let before = runtime_state(&row);

    reconcile_accepted_integrations(&mut connection).unwrap();

    assert_eq!(runtime_state(&row), before);
    assert_attention(&connection);
    assert_no_completion(&connection);
}

#[derive(Clone, Copy, Debug)]
enum CommitTamper {
    Parent,
    Tree,
    Message,
    AuthorName,
    AuthorEmail,
    CommitterName,
    CommitterEmail,
    AuthorDate,
    CommitterDate,
    StoredFingerprint,
}

fn commit_with_metadata(
    row: &Row,
    tree: &str,
    parent: &str,
    message: &str,
    author_name: &str,
    author_email: &str,
    author_date: &str,
    committer_name: &str,
    committer_email: &str,
    committer_date: &str,
) -> String {
    let mut process = Command::new("git")
        .args(["--no-replace-objects", "commit-tree", tree, "-p", parent])
        .current_dir(&row.repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "NUL")
        .env("GIT_AUTHOR_NAME", author_name)
        .env("GIT_AUTHOR_EMAIL", author_email)
        .env("GIT_AUTHOR_DATE", author_date)
        .env("GIT_COMMITTER_NAME", committer_name)
        .env("GIT_COMMITTER_EMAIL", committer_email)
        .env("GIT_COMMITTER_DATE", committer_date)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .as_mut()
        .unwrap()
        .write_all(message.as_bytes())
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    decode_git(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn accepted_integration_exact_commit_tamper_matrix_is_rejected_before_effect() {
    let cases = [
        CommitTamper::Parent,
        CommitTamper::Tree,
        CommitTamper::Message,
        CommitTamper::AuthorName,
        CommitTamper::AuthorEmail,
        CommitTamper::CommitterName,
        CommitTamper::CommitterEmail,
        CommitTamper::AuthorDate,
        CommitTamper::CommitterDate,
        CommitTamper::StoredFingerprint,
    ];
    for case in cases {
        let (_directory, mut connection, row) = fixture();
        let integration = reserve_only(&mut connection, &row);
        let expected_tree = merged_tree(&row, &integration.pre).unwrap();
        let (author_name, author_email, author_date) = author(&row).unwrap();
        let tree = if matches!(case, CommitTamper::Tree) {
            git(
                &row.repo,
                &["rev-parse", &format!("{}^{{tree}}", row.baseline)],
            )
            .unwrap()
        } else {
            expected_tree.clone()
        };
        let parent = if matches!(case, CommitTamper::Parent) {
            row.commit.as_str()
        } else {
            integration.pre.as_str()
        };
        let fingerprint = commit_fingerprint(
            &row,
            &integration,
            &tree,
            &author_name,
            &author_email,
            &author_date,
        );
        let mut message = commit_message(&row, &integration, &tree, &fingerprint);
        if matches!(case, CommitTamper::Message) {
            message.push_str("Tampered-Trailer: yes\n");
        }
        let actual_author_name = if matches!(case, CommitTamper::AuthorName) {
            "Foreign Author"
        } else {
            &author_name
        };
        let actual_author_email = if matches!(case, CommitTamper::AuthorEmail) {
            "foreign-author@example.invalid"
        } else {
            &author_email
        };
        let actual_author_date = if matches!(case, CommitTamper::AuthorDate) {
            "2001-01-01T00:00:00Z"
        } else {
            &author_date
        };
        let actual_committer_name = if matches!(case, CommitTamper::CommitterName) {
            "Foreign Committer"
        } else {
            COMMITTER_NAME
        };
        let actual_committer_email = if matches!(case, CommitTamper::CommitterEmail) {
            "foreign-committer@example.invalid"
        } else {
            COMMITTER_EMAIL
        };
        let actual_committer_date = if matches!(case, CommitTamper::CommitterDate) {
            "2001-01-01T00:00:00Z"
        } else {
            &integration.recorded
        };
        let commit = commit_with_metadata(
            &row,
            &tree,
            parent,
            &message,
            actual_author_name,
            actual_author_email,
            actual_author_date,
            actual_committer_name,
            actual_committer_email,
            actual_committer_date,
        );
        let stored_fingerprint = if matches!(case, CommitTamper::StoredFingerprint) {
            "forged-fingerprint"
        } else {
            &fingerprint
        };
        store_object(
            &connection,
            &integration,
            &commit,
            &tree,
            stored_fingerprint,
        );
        let before = runtime_state(&row);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before, "case={case:?}");
        assert_attention(&connection);
        assert_no_completion(&connection);
    }
}

#[test]
fn accepted_integration_reserved_intent_adopts_preexisting_deterministic_object() {
    let (_directory, mut connection, row) = fixture();
    let integration = reserve_only(&mut connection, &row);
    assert_eq!(integration.stage, "intent_reserved");
    assert!(integration.commit.is_none());
    let tree = merged_tree(&row, &integration.pre).unwrap();
    let (preexisting, _) = create_commit(&row, &integration, &tree).unwrap();
    assert_eq!(
        git(&row.repo, &["cat-file", "-t", &preexisting]).unwrap(),
        "commit"
    );

    reconcile_accepted_integrations(&mut connection).unwrap();

    let stored: String = connection
        .query_row(
            "SELECT integration_commit_id FROM accepted_work_unit_integrations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, preexisting);
    assert_eq!(
        connection
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                row.get(0)
            })
            .unwrap(),
        1
    );
}

#[test]
fn accepted_integration_runtime_and_db_advanced_restart_states_converge_once() {
    {
        let (_directory, mut connection, row) = settled_fixture();
        let commit: String = connection
            .query_row(
                "SELECT integration_commit_id FROM accepted_work_unit_integrations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection
            .execute_batch(
                "DELETE FROM accepted_work_unit_integration_evidence;
                 DELETE FROM work_unit_settlements;
                 DELETE FROM work_unit_prerequisite_contributions;
                 UPDATE accepted_work_unit_integrations
                 SET stage='runtime_advanced',settled_at=NULL,notification_intent_recorded_at=NULL;",
            )
            .unwrap();
        let binding =
            sprint_target_binding_fingerprint(&row.authority, "refs/heads/main", &row.baseline);
        connection
            .execute(
                "UPDATE sprint_target_currents
                 SET current_object_id=?1,binding_fingerprint=?2,version=1",
                params![row.baseline, binding],
            )
            .unwrap();
        let before = runtime_state(&row);
        assert_eq!(before.head, commit);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before);
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
    }
    {
        let (_directory, mut connection, row) = settled_fixture();
        connection
            .execute_batch(
                "DELETE FROM accepted_work_unit_integration_evidence;
                 DELETE FROM work_unit_settlements;
                 DELETE FROM work_unit_prerequisite_contributions;",
            )
            .unwrap();
        rewind_to_db_advanced(&connection);
        let before = runtime_state(&row);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before);
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
    }
}

#[test]
fn accepted_integration_notification_intent_replays_without_delivery() {
    let (_directory, mut connection, _row) = settled_fixture();
    let before: (String, Option<String>) = connection
        .query_row(
            "SELECT notification_intent_recorded_at,notification_delivered_at
             FROM accepted_work_unit_integrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(before.1.is_none());

    reconcile_accepted_integrations(&mut connection).unwrap();

    let after: (String, Option<String>) = connection
        .query_row(
            "SELECT notification_intent_recorded_at,notification_delivered_at
             FROM accepted_work_unit_integrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(
        connection
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                row.get(0)
            })
            .unwrap(),
        1
    );
}

#[test]
fn accepted_integration_divergent_settlement_identity_fails_closed() {
    let mutations = [
        "UPDATE work_unit_settlements SET settlement_id='forged-settlement'",
        "UPDATE work_unit_settlements SET work_unit_id='dependent'",
        "UPDATE work_unit_settlements SET integration_id='forged-integration'",
    ];
    for mutation in mutations {
        let (_directory, mut connection, row) = settled_fixture();
        connection
            .execute_batch(
                "DELETE FROM accepted_work_unit_integration_evidence;
                 DELETE FROM work_unit_prerequisite_contributions;",
            )
            .unwrap();
        rewind_to_db_advanced(&connection);
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch(mutation).unwrap();
        let before = runtime_state(&row);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before, "mutation={mutation}");
        assert_attention(&connection);
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
    }
}

#[test]
fn accepted_integration_divergent_evidence_identity_and_correlation_fail_closed() {
    let mutations = [
        "UPDATE accepted_work_unit_integration_evidence SET evidence_id='forged-evidence'",
        "UPDATE accepted_work_unit_integration_evidence SET integration_id='forged-integration'",
        "UPDATE accepted_work_unit_integration_evidence SET evidence_fingerprint='forged-fingerprint'",
        "UPDATE accepted_work_unit_integration_evidence SET integration_commit_id='0000000000000000000000000000000000000000'",
        "UPDATE accepted_work_unit_integration_evidence SET integration_tree_id='0000000000000000000000000000000000000000'",
        "UPDATE accepted_work_unit_integration_evidence SET parent_object_id='0000000000000000000000000000000000000000'",
        "UPDATE accepted_work_unit_integration_evidence SET candidate_id='forged-candidate'",
        "UPDATE accepted_work_unit_integration_evidence SET target_ref_name='refs/heads/foreign'",
        "UPDATE accepted_work_unit_integration_evidence SET intent_fingerprint='forged-intent'",
    ];
    for mutation in mutations {
        let (_directory, mut connection, row) = settled_fixture();
        connection
            .execute_batch(
                "DELETE FROM work_unit_settlements;
                 DELETE FROM work_unit_prerequisite_contributions;",
            )
            .unwrap();
        rewind_to_db_advanced(&connection);
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch(mutation).unwrap();
        let before = runtime_state(&row);

        reconcile_accepted_integrations(&mut connection).unwrap();

        assert_eq!(runtime_state(&row), before, "mutation={mutation}");
        assert_attention(&connection);
        assert_eq!(
            connection
                .query_row::<i64, _, _>("SELECT COUNT(*) FROM work_unit_settlements", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            0
        );
    }
}

#[test]
fn accepted_integration_contribution_missing_reopens_and_divergence_fails_closed() {
    {
        let (_directory, mut connection, _row) = settled_fixture();
        connection
            .execute_batch("DELETE FROM work_unit_prerequisite_contributions;")
            .unwrap();
        rewind_to_db_advanced(&connection);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(
            connection
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM work_unit_prerequisite_contributions",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row::<String, _, _>(
                    "SELECT stage FROM accepted_work_unit_integrations",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            "settled"
        );
    }
    {
        let (_directory, mut connection, row) = settled_fixture();
        rewind_to_db_advanced(&connection);
        connection
            .execute_batch(
                "INSERT INTO work_units VALUES('extra');
                 INSERT INTO work_unit_relationships VALUES('extra-edge','order','extra','unit');
                 INSERT INTO work_unit_prerequisite_contributions
                 VALUES('extra-contribution','unit','extra',
                   (SELECT integration_id FROM accepted_work_unit_integrations),'extra-edge','t');",
            )
            .unwrap();
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before);
        assert_attention(&connection);
        assert!(connection
            .query_row::<Option<String>, _, _>(
                "SELECT settled_at FROM accepted_work_unit_integrations",
                [],
                |row| row.get(0),
            )
            .unwrap()
            .is_none());
    }
    {
        let (_directory, mut connection, row) = settled_fixture();
        rewind_to_db_advanced(&connection);
        connection
            .execute_batch(
                "UPDATE work_unit_prerequisite_contributions
                 SET prerequisite_work_unit_id='dependent',dependent_work_unit_id='unit';",
            )
            .unwrap();
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before);
        assert_attention(&connection);
    }
    {
        let (_directory, mut connection, row) = settled_fixture();
        rewind_to_db_advanced(&connection);
        connection
            .execute_batch(
                "ALTER TABLE work_unit_prerequisite_contributions RENAME TO legacy_contributions;
                 CREATE TABLE work_unit_prerequisite_contributions(
                   contribution_id TEXT,prerequisite_work_unit_id TEXT,dependent_work_unit_id TEXT,
                   integration_id TEXT,relationship_id TEXT,recorded_at TEXT
                 );
                 INSERT INTO work_unit_prerequisite_contributions
                   SELECT * FROM legacy_contributions;
                 INSERT INTO work_unit_prerequisite_contributions
                   SELECT contribution_id || '-duplicate',prerequisite_work_unit_id,
                          dependent_work_unit_id,integration_id,relationship_id,recorded_at
                   FROM legacy_contributions;
                 DROP TABLE legacy_contributions;",
            )
            .unwrap();
        let before = runtime_state(&row);
        reconcile_accepted_integrations(&mut connection).unwrap();
        assert_eq!(runtime_state(&row), before);
        assert_attention(&connection);
    }
}
