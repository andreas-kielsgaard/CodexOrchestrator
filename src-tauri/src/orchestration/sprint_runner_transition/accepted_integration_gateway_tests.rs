use super::{fingerprint_bytes, SCHEMA};
use crate::orchestration::{
    accepted_candidate_authority::reconcile_accepted_candidate_authorities,
    accepted_integration::reconcile_accepted_integrations,
};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn run(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "NUL")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["--no-replace-objects"])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "NUL")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn durable_fingerprint(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

struct FullGatewayFixture {
    _directory: TempDir,
    connection: Connection,
    repository: std::path::PathBuf,
    attempt_worktree: std::path::PathBuf,
    baseline: String,
    candidate: String,
}

impl FullGatewayFixture {
    fn new() -> Self {
        let directory = TempDir::new().unwrap();
        let repository = directory.path().join("repository");
        let attempt_worktree = directory.path().join("attempt");
        fs::create_dir(&repository).unwrap();
        run(&repository, &["init", "-b", "main"]);
        run(&repository, &["config", "user.name", "Test User"]);
        run(
            &repository,
            &["config", "user.email", "test@example.invalid"],
        );
        fs::write(repository.join("base.txt"), "base\n").unwrap();
        run(&repository, &["add", "."]);
        run(&repository, &["commit", "-m", "base"]);
        let baseline = git(&repository, &["rev-parse", "HEAD"]);
        let attempt_route = attempt_worktree.to_string_lossy().to_string();
        run(
            &repository,
            &["worktree", "add", "-b", "candidate", &attempt_route, "main"],
        );
        fs::write(attempt_worktree.join("base.txt"), "candidate\n").unwrap();
        fs::write(attempt_worktree.join("candidate.txt"), "candidate\n").unwrap();
        run(&attempt_worktree, &["add", "."]);
        run(&attempt_worktree, &["commit", "-m", "candidate"]);
        let candidate = git(&attempt_worktree, &["rev-parse", "HEAD"]);

        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        connection.execute_batch(SCHEMA).unwrap();

        let payload = serde_json::to_vec(&serde_json::json!({
            "files": [{
                "changedFileReferenceId": "e1",
                "content": {"encoding": "base64", "bytesBase64": "Y2FuZGlkYXRlCg=="}
            }]
        }))
        .unwrap();
        let artifact: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        let manifest = serde_json::json!([{
            "evidenceRef": "e1",
            "displayName": "candidate.txt",
            "changeKind": "added"
        }])
        .to_string();
        let content_fingerprints = serde_json::json!([{
            "evidenceRef": "e1",
            "contentFingerprint": fingerprint_bytes(
                "implementer-evidence-content",
                &serde_json::to_vec(&artifact["files"][0]).unwrap()
            )
        }])
        .to_string();
        let comparison = fingerprint_bytes("implementer-evidence-comparison", &payload);
        let common = repository.join(".git");
        let repository_route = repository.to_string_lossy().to_string();
        let attempt_route = attempt_worktree.to_string_lossy().to_string();
        let capture_fingerprint = durable_fingerprint(&[
            "capture",
            "capture-key",
            "epic",
            "sprint",
            "provenance",
            "repository-id",
            &repository_route,
            "attempt-worktree",
            &attempt_route,
            &baseline,
            &candidate,
        ]);

        connection
            .execute_batch(
                "INSERT INTO initiated_sprints
                   (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,
                    sprint_plan_id,sprint_plan_revision_id)
                 VALUES('sprint','epic',0,'Sprint','Integrate','[]','plan','plan-revision');
                 INSERT INTO work_unit_materializations
                   (materialization_id,planning_point_id,accepted_revision_id,epic_id,sprint_id,
                    work_slice_id,authorization_recorded_at)
                 VALUES('materialization','planning-point','accepted-revision','epic','sprint',
                        'work-slice','t');
                 INSERT INTO work_units
                   (work_unit_id,materialization_id,work_slice_id,accepted_revision_id,lane_ordinal,
                    lane_title,specification)
                 VALUES('unit','materialization','work-slice','accepted-revision',0,'Lane','Do work');
                 INSERT INTO work_unit_handler_activations
                   (work_unit_id,materialization_id,sprint_id,attempt_id,handler_session_id,
                    handler_invocation_id,handler_harness_key,handler_harness_version,
                    eligibility_state,requested_at)
                 VALUES('unit','materialization','sprint','attempt','handler-session',
                        'handler-invocation','handler',1,'eligible','t');
                 INSERT INTO work_unit_handler_reviews
                   (work_unit_id,attempt_id,reporting_invocation_id,handler_session_id,
                    original_handler_invocation_id,action_handler_invocation_id,
                    review_invocation_id,review_harness_revision_id,
                    review_harness_configuration_digest,review_harness_repository_commit_ref,
                    delivery_requested_at,delivered_payload_json,delivered_payload_fingerprint,
                    semantic_judgment_variant,lifecycle_status)
                 VALUES('unit','attempt','reporting','handler-session','handler-invocation',
                        'action-invocation','review','review-revision','review-digest','review-commit',
                        't','{}','delivery','accept','completed');
                 INSERT INTO work_unit_handler_decisions
                   (work_unit_id,review_invocation_id,decision_variant,decision_fingerprint,
                    decision_recorded_at,implementation_accepted_at)
                 VALUES('unit','review','accepted','decision','t','t');
                 INSERT INTO execution_support_grants
                   (attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,
                    workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at)
                 VALUES('attempt','capability','epic','sprint','unit','repository-id',
                        'work_unit_implementer','attempt-worktree','workspace-fingerprint',
                        'correlation-fingerprint','t');
                 INSERT INTO file_review_documents
                   (document_ref_id,epic_id,sprint_id,provenance_id,opaque_reference,title,
                    idempotency_key,payload_fingerprint,recorded_at)
                 VALUES('document','epic','sprint','provenance','opaque','Evidence',
                        'document-key','document-fingerprint','t');
                 INSERT INTO file_review_changed_files
                   (document_ref_id,changed_file_reference_id,display_name,change_kind,ordinal)
                 VALUES('document','e1','candidate.txt','added',0);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO initiated_sprint_git_authorities
                   (authority_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,
                    provenance_id,repository_id,repository_root,repository_common_dir,worktree_id,
                    worktree_root,baseline_object_id,current_object_id,runtime_instance_ref,
                    runtime_source_ref,source_fingerprint,recorded_at)
                 VALUES('authority','authority-key','authority-fingerprint','epic','sprint',
                        'provenance','repository-id',?1,?2,'target-worktree',?1,?3,?3,
                        'runtime-instance','refs/heads/main','source-fingerprint','t')",
                params![
                    repository.to_string_lossy(),
                    common.to_string_lossy(),
                    baseline
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_support_attempt_authorizations
                   (attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,
                    authorization_fingerprint,recorded_at)
                 VALUES('attempt','unit','work_unit_implementer','authority',?1,
                        'authorization-fingerprint','t')",
                [&baseline],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO file_review_git_capture_authorizations
                   (capture_authorization_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,
                    provenance_id,repository_id,repository_root,worktree_id,worktree_root,
                    baseline_object_id,current_object_id,recorded_at)
                 VALUES('capture','capture-key',?1,'epic','sprint','provenance',
                        'repository-id',?2,'attempt-worktree',?3,?4,?5,'t')",
                params![
                    capture_fingerprint,
                    repository_route,
                    attempt_route,
                    baseline,
                    candidate
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO stored_file_review_artifacts
                   (artifact_id,document_ref_id,contract_version,payload,payload_bytes,provenance_id)
                 VALUES('artifact','document','stored-file-review-artifact/v1',?1,?2,'provenance')",
                params![payload, payload.len() as i64],
            )
            .unwrap();
        connection
            .execute_batch(
                "INSERT INTO file_review_git_capture_documents
                   (capture_authorization_id,document_ref_id,artifact_id,linkage_fingerprint,recorded_at)
                 VALUES('capture','document','artifact','linkage-fingerprint','t');",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_unit_implementer_outcomes
                   (work_unit_id,attempt_id,implementer_session_id,implementer_invocation_id,
                    reporting_invocation_id,reporting_harness_revision_id,
                    reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,
                    reporting_requested_at,evidence_manifest_json,comparison_fingerprint,
                    evidence_content_fingerprints_json,file_review_capture_authorization_id,
                    evidence_ready_at,application_accepted_at)
                 VALUES('unit','attempt','implementer-session','implementer-invocation','reporting',
                        'reporting-revision','reporting-digest','reporting-commit','t',?1,?2,?3,
                        'capture','t','t')",
                params![manifest, comparison, content_fingerprints],
            )
            .unwrap();

        Self {
            _directory: directory,
            connection,
            repository,
            attempt_worktree,
            baseline,
            candidate,
        }
    }

    fn pin_candidate(&mut self) -> String {
        reconcile_accepted_candidate_authorities(&mut self.connection).unwrap();
        reconcile_accepted_candidate_authorities(&mut self.connection).unwrap();
        let candidate_id: String = self
            .connection
            .query_row(
                "SELECT candidate_id FROM accepted_handler_candidates WHERE pinned_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            git(
                &self.repository,
                &[
                    "show-ref",
                    "--verify",
                    "--hash",
                    &format!("refs/codex/orchestrator/accepted/{candidate_id}"),
                ],
            ),
            self.candidate
        );
        candidate_id
    }

    fn remove_attempt_worktree(&self) {
        let attempt_route = self.attempt_worktree.to_string_lossy().to_string();
        run(
            &self.repository,
            &["worktree", "remove", "--force", &attempt_route],
        );
        assert!(!self.attempt_worktree.exists());
    }
}

#[test]
fn accepted_integration_full_gateway_revalidates_retained_lineage_without_attempt_worktree() {
    let mut fixture = FullGatewayFixture::new();
    let candidate_id = fixture.pin_candidate();
    fixture.remove_attempt_worktree();

    reconcile_accepted_integrations(&mut fixture.connection).unwrap();
    reconcile_accepted_integrations(&mut fixture.connection).unwrap();

    let integration_commit: String = fixture
        .connection
        .query_row(
            "SELECT integration_commit_id FROM accepted_work_unit_integrations
             WHERE candidate_id=?1 AND stage='settled' AND attention_code IS NULL",
            [&candidate_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        git(&fixture.repository, &["rev-parse", "HEAD"]),
        integration_commit
    );
    assert_eq!(
        git(
            &fixture.repository,
            &["rev-list", "--parents", "-n", "1", &integration_commit],
        ),
        format!("{integration_commit} {}", fixture.baseline)
    );
    for table in [
        "accepted_work_unit_integration_evidence",
        "work_unit_settlements",
    ] {
        assert_eq!(
            fixture
                .connection
                .query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                    .get(0),)
                .unwrap(),
            1
        );
    }
}

#[test]
fn accepted_integration_full_gateway_artifact_content_capture_and_baseline_tamper_fail_closed() {
    let mutations = [
        "UPDATE stored_file_review_artifacts SET payload=x'7B2266696C6573223A5B5D7D'",
        "UPDATE work_unit_implementer_outcomes SET evidence_content_fingerprints_json='[]'",
        "UPDATE file_review_git_capture_authorizations SET payload_fingerprint='forged-capture'",
        "UPDATE accepted_handler_candidates SET attempt_baseline_object_id=candidate_commit_id",
    ];
    for mutation in mutations {
        let mut fixture = FullGatewayFixture::new();
        let candidate_id = fixture.pin_candidate();
        fixture.remove_attempt_worktree();
        let before = git(&fixture.repository, &["rev-parse", "HEAD"]);
        fixture.connection.execute_batch(mutation).unwrap();

        reconcile_accepted_integrations(&mut fixture.connection).unwrap();

        assert_eq!(git(&fixture.repository, &["rev-parse", "HEAD"]), before);
        assert_eq!(before, fixture.baseline);
        assert_eq!(
            fixture
                .connection
                .query_row::<String, _, _>(
                    "SELECT stage FROM accepted_work_unit_integrations WHERE candidate_id=?1",
                    [&candidate_id],
                    |row| row.get(0),
                )
                .unwrap(),
            "attention",
            "mutation={mutation}"
        );
        for table in [
            "accepted_work_unit_integration_evidence",
            "work_unit_settlements",
            "work_unit_prerequisite_contributions",
        ] {
            assert_eq!(
                fixture
                    .connection
                    .query_row::<i64, _, _>(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get(0),)
                    .unwrap(),
                0,
                "table={table} mutation={mutation}"
            );
        }
    }
}
