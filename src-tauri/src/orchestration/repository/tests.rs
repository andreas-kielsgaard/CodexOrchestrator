use super::*;
use crate::orchestration::{
    application::OrchestrationApplication,
    domain::{
        CapabilityProfileId, EpicPlanningDraftId, PlanBuilderProposal,
        PlanningDraftAgentSessionAssociationId, ProposedSprint,
    },
    file_review_git_producer::{
        produce_file_review_from_git, FileReviewGitProducerError, ProduceFileReviewFromGit,
    },
};
use chrono::{TimeZone, Utc};
use std::{
    env, fs,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};

const SESSION_ID: &str = "agent-session-plan-builder";
const ASSOCIATION_ID: &str = "planning-draft-agent-session-association-1";
const ACTOR_ID: &str = "managed-plan-builder";

#[test]
fn file_review_store_replays_exact_facts_and_reauthorizes_on_load() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "review-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "review-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let epic = &query.initiated_epics[0];
    let facts = StoreFileReviewFacts {
        document_ref_id: "review-document".into(),
        epic_id: epic.epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: epic.provenance_id.clone(),
        opaque_reference: "opaque-review".into(),
        title: "Changed files".into(),
        summary: None,
        artifact_id: "review-artifact".into(),
        payload: b"{}".to_vec(),
        idempotency_key: "review-store".into(),
        changed_files: vec![FileReviewChangedFileWrite {
            changed_file_reference_id: "changed-1".into(),
            display_name: "src/a.ts".into(),
            change_kind: "modified".into(),
            previous_display_name: None,
        }],
    };
    assert_eq!(
        repository.store_file_review_facts(facts.clone()).unwrap(),
        StoreFileReviewFactsResult::Stored
    );
    assert_eq!(
        repository.store_file_review_facts(facts).unwrap(),
        StoreFileReviewFactsResult::IdempotentReplay
    );
    match repository.load_scoped_file_review("opaque-review").unwrap() {
        ScopedFileReviewLoad::Available { document } => {
            assert_eq!(document.artifact_id, "review-artifact");
            assert_eq!(
                document.changed_files[0].changed_file_reference_id,
                "changed-1"
            );
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn git_capture_authorization_is_private_replay_safe_and_reauthorized() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let epic = &query.initiated_epics[0];
    let value = FileReviewGitCaptureAuthorizationWrite {
        capture_authorization_id: "capture-1".into(),
        idempotency_key: "capture-key".into(),
        epic_id: epic.epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: epic.provenance_id.clone(),
        repository_id: "repository-1".into(),
        repository_root: "C:\\repo".into(),
        worktree_id: "worktree-1".into(),
        worktree_root: "C:\\repo\\worktree".into(),
        baseline_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        current_object_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
    };
    assert_eq!(
        repository
            .store_file_review_git_capture_authorization(value.clone())
            .unwrap(),
        StoreFileReviewGitCaptureAuthorizationResult::Stored
    );
    assert_eq!(
        repository
            .store_file_review_git_capture_authorization(value.clone())
            .unwrap(),
        StoreFileReviewGitCaptureAuthorizationResult::IdempotentReplay
    );
    let loaded = repository
        .load_file_review_git_capture_authorization("capture-1")
        .unwrap()
        .unwrap();
    assert_eq!(loaded.worktree_root, "C:\\repo\\worktree");
    assert!(!serde_json::to_string(&repository.native_query().unwrap())
        .unwrap()
        .contains("C:\\repo"));
    let mut conflict = value;
    conflict.current_object_id = "cccccccccccccccccccccccccccccccccccccccc".into();
    assert!(matches!(
        repository.store_file_review_git_capture_authorization(conflict),
        Err(FileReviewGitCaptureAuthorizationError::Conflict)
    ));
}

#[test]
fn git_producer_persists_complete_ordered_real_object_facts_and_replays_exactly() {
    let git = real_file_review_repository();
    let repository = initiated_repository_with_capture("capture-real", &git);

    let first = produce_file_review_from_git(
        &repository,
        ProduceFileReviewFromGit {
            capture_authorization_id: "capture-real".into(),
        },
    )
    .expect("produce");
    assert!(!first.idempotent_replay);
    assert_eq!(first.changed_file_count, 6);

    let document = match repository
        .load_scoped_file_review(&first.opaque_reference)
        .expect("scoped load")
    {
        ScopedFileReviewLoad::Available { document } => document,
        other => panic!("unexpected scoped load: {other:?}"),
    };
    assert_eq!(document.document_ref_id, first.document_ref_id);
    assert_eq!(document.artifact_id, first.artifact_id);
    let paths = document
        .changed_files
        .iter()
        .map(|file| file.display_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "added.md",
            "binary.bin",
            "deleted.md",
            "docs/new-name.md",
            "modified.txt",
            "unsupported.dat",
        ]
    );
    let renamed = &document.changed_files[3];
    assert_eq!(renamed.change_kind, "renamed");
    assert_eq!(
        renamed.previous_display_name.as_deref(),
        Some("docs/old-name.md")
    );

    let payload: serde_json::Value = serde_json::from_slice(&document.payload).expect("payload");
    assert_eq!(payload["contractVersion"], STORED_FILE_REVIEW_ARTIFACT_V1);
    assert_eq!(payload["documentRefId"], first.document_ref_id);
    assert_eq!(payload["artifactId"], first.artifact_id);
    let files = payload["files"].as_array().expect("files");
    assert_eq!(files.len(), document.changed_files.len());
    for (payload_file, membership) in files.iter().zip(&document.changed_files) {
        assert_eq!(
            payload_file["changedFileReferenceId"],
            membership.changed_file_reference_id
        );
    }
    let encoding_for = |path: &str| {
        let index = document
            .changed_files
            .iter()
            .position(|file| file.display_name == path)
            .expect("membership");
        files[index]["content"]["encoding"]
            .as_str()
            .expect("encoding")
    };
    assert_eq!(encoding_for("added.md"), "utf-8");
    assert_eq!(encoding_for("binary.bin"), "binary");
    assert_eq!(encoding_for("unsupported.dat"), "unsupported");
    assert!(!files[0]["hunks"].as_array().expect("text hunks").is_empty());
    assert!(files[1]["hunks"]
        .as_array()
        .expect("binary hunks")
        .is_empty());

    let query_json = serde_json::to_string(&repository.native_query().expect("native query"))
        .expect("query json");
    assert!(query_json.contains(&first.document_ref_id));
    assert!(!query_json.contains("capture-real"));
    assert!(!query_json.contains(git.root.to_string_lossy().as_ref()));

    let replay = produce_file_review_from_git(
        &repository,
        ProduceFileReviewFromGit {
            capture_authorization_id: "capture-real".into(),
        },
    )
    .expect("replay");
    assert!(replay.idempotent_replay);
    assert_eq!(replay.document_ref_id, first.document_ref_id);
    assert_eq!(replay.artifact_id, first.artifact_id);
    assert_eq!(replay.opaque_reference, first.opaque_reference);
}

#[test]
fn git_producer_ignores_hostile_inherited_git_environment() {
    const CHILD: &str = "FILE_REVIEW_HOSTILE_GIT_CHILD";
    if env::var_os(CHILD).is_some() {
        let root = std::path::PathBuf::from(env::var("FILE_REVIEW_TEST_ROOT").unwrap());
        let authorized = RealGitRepository {
            root,
            _temp: tempfile::tempdir().unwrap(),
            baseline: env::var("FILE_REVIEW_TEST_BASELINE").unwrap(),
            current: env::var("FILE_REVIEW_TEST_CURRENT").unwrap(),
        };
        let repository = initiated_repository_with_capture("capture-hostile", &authorized);
        let first = produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-hostile".into(),
            },
        )
        .expect("hostile-environment production");
        let first_document = match repository
            .load_scoped_file_review(&first.opaque_reference)
            .unwrap()
        {
            ScopedFileReviewLoad::Available { document } => document,
            other => panic!("unexpected first load: {other:?}"),
        };
        let replay = produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-hostile".into(),
            },
        )
        .expect("hostile-environment replay");
        let replay_document = match repository
            .load_scoped_file_review(&replay.opaque_reference)
            .unwrap()
        {
            ScopedFileReviewLoad::Available { document } => document,
            other => panic!("unexpected replay load: {other:?}"),
        };
        assert!(replay.idempotent_replay);
        assert_eq!(replay.document_ref_id, first.document_ref_id);
        assert_eq!(replay.artifact_id, first.artifact_id);
        assert_eq!(replay.opaque_reference, first.opaque_reference);
        assert_eq!(replay_document, first_document);
        return;
    }

    let authorized = real_file_review_repository();
    let hostile = real_file_review_repository();
    let evidence = tempfile::tempdir().unwrap();
    let trace = evidence.path().join("ambient-git-trace.log");
    let trace2 = evidence.path().join("ambient-git-trace2.json");
    let global_config = evidence.path().join("hostile.gitconfig");
    fs::write(&global_config, b"[core]\n\tbare = true\n").unwrap();
    let status = Command::new(env::current_exe().unwrap())
        .args([
            "--exact",
            "orchestration::repository::tests::git_producer_ignores_hostile_inherited_git_environment",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("FILE_REVIEW_TEST_ROOT", canonical_root(&authorized.root))
        .env("FILE_REVIEW_TEST_BASELINE", &authorized.baseline)
        .env("FILE_REVIEW_TEST_CURRENT", &authorized.current)
        .env("GIT_TRACE", &trace)
        .env("GIT_TRACE2_EVENT", &trace2)
        .env("GIT_DIR", hostile.root.join(".git"))
        .env("GIT_WORK_TREE", &hostile.root)
        .env("GIT_CONFIG_GLOBAL", &global_config)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.bare")
        .env("GIT_CONFIG_VALUE_0", "true")
        .env("GIT_EXEC_PATH", &hostile.root)
        .status()
        .expect("hostile child test");
    assert!(status.success());
    assert!(
        !trace.exists(),
        "ambient GIT_TRACE created an external file"
    );
    assert!(
        !trace2.exists(),
        "ambient GIT_TRACE2_EVENT created an external file"
    );
}

#[test]
fn git_producer_denies_missing_tampered_object_and_repository_identity() {
    let git = real_file_review_repository();
    let repository = initiated_repository_with_capture("capture-denial", &git);
    let other_epic_provenance = initiate_second_epic(&repository);
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: " ".into(),
            }
        ),
        Err(FileReviewGitProducerError::InvalidRequest)
    );
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "missing".into(),
            }
        ),
        Err(FileReviewGitProducerError::Unauthorized)
    );

    repository
        .lock()
        .unwrap()
        .execute(
            "UPDATE file_review_git_capture_authorizations SET current_object_id=?1 WHERE capture_authorization_id='capture-denial'",
            ["cccccccccccccccccccccccccccccccccccccccc"],
        )
        .unwrap();
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-denial".into(),
            }
        ),
        Err(FileReviewGitProducerError::GitObjectUnavailable)
    );

    let other = real_file_review_repository();
    repository
        .lock()
        .unwrap()
        .execute(
            "UPDATE file_review_git_capture_authorizations SET current_object_id=?1, worktree_root=?2 WHERE capture_authorization_id='capture-denial'",
            params![git.current, canonical_root(&other.root)],
        )
        .unwrap();
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-denial".into(),
            }
        ),
        Err(FileReviewGitProducerError::RepositoryMismatch)
    );

    repository
        .lock()
        .unwrap()
        .execute(
            "UPDATE file_review_git_capture_authorizations SET provenance_id=?1 WHERE capture_authorization_id='capture-denial'",
            [&other_epic_provenance],
        )
        .unwrap();
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-denial".into(),
            }
        ),
        Err(FileReviewGitProducerError::Unauthorized)
    );
}

#[test]
fn git_producer_fails_closed_on_count_size_and_conflicting_replay() {
    let oversized = tempfile::tempdir().expect("temp repo");
    init_git(oversized.path());
    write_git_file(oversized.path(), "small.txt", b"baseline\n");
    git(oversized.path(), &["add", "-A"]);
    git(oversized.path(), &["commit", "-m", "baseline"]);
    let baseline = git_text(oversized.path(), &["rev-parse", "HEAD"]);
    write_git_file(oversized.path(), "small.txt", &vec![b'x'; 256_001]);
    git(oversized.path(), &["add", "-A"]);
    git(oversized.path(), &["commit", "-m", "current"]);
    let current = git_text(oversized.path(), &["rev-parse", "HEAD"]);
    let oversized_git = RealGitRepository {
        root: oversized.path().to_path_buf(),
        _temp: oversized,
        baseline,
        current,
    };
    let oversized_repository =
        initiated_repository_with_capture("capture-oversized", &oversized_git);
    assert_eq!(
        produce_file_review_from_git(
            &oversized_repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-oversized".into(),
            }
        ),
        Err(FileReviewGitProducerError::LimitsExceeded)
    );
    assert!(oversized_repository
        .native_query()
        .unwrap()
        .file_review_documents
        .is_empty());

    let excessive = tempfile::tempdir().expect("temp repo");
    init_git(excessive.path());
    git(
        excessive.path(),
        &["commit", "--allow-empty", "-m", "baseline"],
    );
    let baseline = git_text(excessive.path(), &["rev-parse", "HEAD"]);
    for index in 0..=500 {
        write_git_file(
            excessive.path(),
            &format!("many/file-{index:03}.txt"),
            b"changed\n",
        );
    }
    git(excessive.path(), &["add", "-A"]);
    git(excessive.path(), &["commit", "-m", "current"]);
    let current = git_text(excessive.path(), &["rev-parse", "HEAD"]);
    let excessive_git = RealGitRepository {
        root: excessive.path().to_path_buf(),
        _temp: excessive,
        baseline,
        current,
    };
    let excessive_repository =
        initiated_repository_with_capture("capture-excessive", &excessive_git);
    assert_eq!(
        produce_file_review_from_git(
            &excessive_repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-excessive".into(),
            }
        ),
        Err(FileReviewGitProducerError::LimitsExceeded)
    );
    assert!(excessive_repository
        .native_query()
        .unwrap()
        .file_review_documents
        .is_empty());

    let git = real_file_review_repository();
    let repository = initiated_repository_with_capture("capture-conflict", &git);
    let first = produce_file_review_from_git(
        &repository,
        ProduceFileReviewFromGit {
            capture_authorization_id: "capture-conflict".into(),
        },
    )
    .unwrap();
    let connection = repository.lock().unwrap();
    connection
        .execute(
            "UPDATE file_review_documents SET payload_fingerprint='tampered' WHERE document_ref_id=?1",
            [&first.document_ref_id],
        )
        .unwrap();
    let counts = |connection: &Connection| {
        [
            "file_review_documents",
            "file_review_changed_files",
            "stored_file_review_artifacts",
        ]
        .map(|table| {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        })
    };
    let before = counts(&connection);
    drop(connection);
    assert_eq!(
        produce_file_review_from_git(
            &repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: "capture-conflict".into(),
            }
        ),
        Err(FileReviewGitProducerError::Conflict)
    );
    let after = counts(&repository.lock().unwrap());
    assert_eq!(after, before);
}

#[test]
fn file_review_store_rejects_wrong_provenance_without_writing() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "review-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "review-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let result = repository.store_file_review_facts(StoreFileReviewFacts {
        document_ref_id: "bad-document".into(),
        epic_id: query.initiated_epics[0].epic_id.clone(),
        sprint_id: query.initiated_sprints[0].sprint_id.clone(),
        provenance_id: "wrong-provenance".into(),
        opaque_reference: "opaque-bad".into(),
        title: "Changed files".into(),
        summary: None,
        artifact_id: "bad-artifact".into(),
        payload: b"{}".to_vec(),
        idempotency_key: "bad-store".into(),
        changed_files: vec![FileReviewChangedFileWrite {
            changed_file_reference_id: "changed-1".into(),
            display_name: "src/a.ts".into(),
            change_kind: "modified".into(),
            previous_display_name: None,
        }],
    });
    assert!(matches!(result, Err(FileReviewFactsError::Forbidden)));
    assert!(matches!(
        repository.load_scoped_file_review("opaque-bad").unwrap(),
        ScopedFileReviewLoad::Unavailable
    ));
}

#[test]
fn scoped_file_review_distinguishes_unknown_invalid_and_broken_membership() {
    let repository = repository_at(time());
    assert!(matches!(
        repository.load_scoped_file_review("missing").unwrap(),
        ScopedFileReviewLoad::Unavailable
    ));
    assert!(matches!(
        repository.load_scoped_file_review(" ").unwrap(),
        ScopedFileReviewLoad::Invalid
    ));
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("review"), "status-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "status-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    repository
        .store_file_review_facts(StoreFileReviewFacts {
            document_ref_id: "status-document".into(),
            epic_id: query.initiated_epics[0].epic_id.clone(),
            sprint_id: query.initiated_sprints[0].sprint_id.clone(),
            provenance_id: query.initiated_epics[0].provenance_id.clone(),
            opaque_reference: "opaque-status".into(),
            title: "Changed files".into(),
            summary: None,
            artifact_id: "status-artifact".into(),
            payload: b"{}".to_vec(),
            idempotency_key: "status-store".into(),
            changed_files: vec![FileReviewChangedFileWrite {
                changed_file_reference_id: "changed-1".into(),
                display_name: "src/a.ts".into(),
                change_kind: "modified".into(),
                previous_display_name: None,
            }],
        })
        .unwrap();
    repository
        .lock()
        .unwrap()
        .execute(
            "DELETE FROM stored_file_review_artifacts WHERE artifact_id='status-artifact'",
            [],
        )
        .unwrap();
    assert!(matches!(
        repository.load_scoped_file_review("opaque-status").unwrap(),
        ScopedFileReviewLoad::Unauthorized
    ));
}

#[test]
fn file_review_rejects_a_valid_other_epic_provenance_and_omits_it_from_query() {
    let repository = repository_at(time());
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "one-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: first.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "one-init".into(),
        })
        .unwrap();
    let draft = EpicPlanningDraftId::new("epic-planning-draft-2").unwrap();
    let profile = CapabilityProfileId::new("capability-profile-2").unwrap();
    let association =
        PlanningDraftAgentSessionAssociationId::new("planning-draft-agent-session-association-2")
            .unwrap();
    repository.create_planning_draft(&draft, time()).unwrap();
    repository
        .create_capability_profile(&profile, "active", time())
        .unwrap();
    repository
        .associate_agent_session(&association, &draft, SESSION_ID, ACTOR_ID, time())
        .unwrap();
    repository
        .assign_profile(&draft, &profile, &association, at(2030, 1, 1), time())
        .unwrap();
    let second = repository
        .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
            epic_planning_draft_id: draft.clone(),
            capability_profile_id: profile,
            agent_session_association_id: association,
            agent_session_id: SESSION_ID.into(),
            actor_id: ACTOR_ID.into(),
            expected_revision: None,
            proposal: proposal("two"),
            idempotency_key: "two-save".into(),
        })
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: draft,
            expected_revision_token: second.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "two-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let first_epic = &query.initiated_epics[0];
    repository
        .store_file_review_facts(StoreFileReviewFacts {
            document_ref_id: "tamper-document".into(),
            epic_id: first_epic.epic_id.clone(),
            sprint_id: query
                .initiated_sprints
                .iter()
                .find(|x| x.epic_id == first_epic.epic_id)
                .unwrap()
                .sprint_id
                .clone(),
            provenance_id: first_epic.provenance_id.clone(),
            opaque_reference: "opaque-tamper".into(),
            title: "Changed files".into(),
            summary: None,
            artifact_id: "tamper-artifact".into(),
            payload: b"{}".to_vec(),
            idempotency_key: "tamper-store".into(),
            changed_files: vec![FileReviewChangedFileWrite {
                changed_file_reference_id: "changed".into(),
                display_name: "src/a.ts".into(),
                change_kind: "modified".into(),
                previous_display_name: None,
            }],
        })
        .unwrap();
    let other = query
        .initiated_epics
        .iter()
        .find(|x| x.epic_id != first_epic.epic_id)
        .unwrap()
        .provenance_id
        .clone();
    let connection = repository.lock().unwrap();
    connection.execute("UPDATE file_review_documents SET provenance_id=?1 WHERE document_ref_id='tamper-document'", [&other]).unwrap();
    connection.execute("UPDATE stored_file_review_artifacts SET provenance_id=?1 WHERE artifact_id='tamper-artifact'", [&other]).unwrap();
    drop(connection);
    assert!(matches!(
        repository.load_scoped_file_review("opaque-tamper").unwrap(),
        ScopedFileReviewLoad::Unauthorized
    ));
    assert!(!serde_json::to_string(&repository.native_query().unwrap())
        .unwrap()
        .contains("tamper-document"));
}

#[test]
fn initiation_is_atomic_idempotent_and_preserves_the_consumed_revision() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("init"), "save"))
        .unwrap();
    let initiate = |key: &str, token: String| super::super::domain::InitiateEpicCommand {
        epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
        expected_revision_token: token,
        actor_id: "application-user".into(),
        idempotency_key: key.into(),
    };
    let result = repository
        .initiate_epic(initiate("init", saved.revision_token.clone()))
        .unwrap();
    let retry = repository
        .initiate_epic(initiate("init", saved.revision_token.clone()))
        .unwrap();
    assert_eq!(result.epic_id, retry.epic_id);
    assert!(retry.idempotent_replay);
    assert!(matches!(
        repository.initiate_epic(initiate("init", "different-revision-token".into())),
        Err(super::super::domain::InitiateEpicError::IdempotencyConflict)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate("other", saved.revision_token)),
        Err(super::super::domain::InitiateEpicError::AlreadyInitiated)
    ));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("after"), "after")),
        Err(SaveProposalError::DraftNotFound)
    ));
    let query = repository.native_query().expect("query");
    assert_eq!(query.planning_drafts[0].status, "initiated");
    let connection = repository.lock().unwrap();
    for table in [
        "epic_initiation_commands",
        "epic_initiation_results",
        "epic_initiation_events",
        "epic_initiation_provenance",
        "epic_initiation_material_snapshots",
        "epic_initiations",
        "initiated_sprints",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            if table == "initiated_sprints" { 1 } else { 1 }
        );
    }
}

#[test]
fn button_context_claim_keeps_stable_invocation_identity_until_explicit_reconciliation() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("context"), "context-save"))
        .unwrap();
    let initiation = repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "context-initiation".into(),
        })
        .unwrap();
    repository
        .schedule_button_initiation_context(&initiation)
        .unwrap();
    repository
        .schedule_button_initiation_context(&initiation)
        .unwrap();

    let first = repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "claim-before-restart",
            "context-invocation-before-restart",
        )
        .unwrap()
        .unwrap();
    assert!(repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "claim-after-restart",
            "must-not-replace-target",
        )
        .is_err());
    let recovered = repository
        .load_claimed_plan_builder_context(SESSION_ID)
        .unwrap()
        .unwrap();
    assert_eq!(first, recovered);
    assert_eq!(recovered.initiation_id, initiation.initiation_id.as_str());
    assert_eq!(recovered.epic_id, initiation.epic_id.as_str());

    repository.release_plan_builder_context(&recovered).unwrap();
    let delivery = repository
        .claim_pending_plan_builder_context(SESSION_ID, "delivery-claim", "context-invocation")
        .unwrap()
        .unwrap();
    repository
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO agent_session_invocations
             (id,session_id,submitted_text,input_provenance,status,requested_options_json,started_at,created_at,updated_at)
             VALUES ('context-invocation',?1,'original application text','application','running','{}','t','t','t')",
            params![SESSION_ID],
        )
        .unwrap();
    repository.consume_plan_builder_context(&delivery).unwrap();
    assert!(repository
        .claim_pending_plan_builder_context(
            SESSION_ID,
            "must-not-redeliver",
            "must-not-redeliver-invocation",
        )
        .unwrap()
        .is_none());
    let facts: (String, String, String, String, String) = repository
        .lock()
        .unwrap()
        .query_row(
            "SELECT source_kind,requested_at,pending_at,delivered_at,consumed_at
             FROM plan_builder_context_deliveries",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(facts.0, "button_initiation");
    assert!(facts.1 <= facts.2 && facts.2 <= facts.3 && facts.3 <= facts.4);
}

#[test]
fn initiated_draft_is_terminal_for_managed_draft_mutations() {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("terminal"), "terminal-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "terminal-initiation".into(),
        })
        .unwrap();
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").unwrap();

    assert!(matches!(
        repository.update_planning_draft_title(
            &draft,
            SESSION_ID,
            Some("Late title"),
            "late-title"
        ),
        Err(SaveProposalError::Forbidden)
    ));
    assert!(matches!(
        repository.cancel_planning_draft(&draft, SESSION_ID, "late-cancel"),
        Err(SaveProposalError::Forbidden)
    ));
    assert!(repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .is_ok());
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.planning_drafts[0].status, "initiated");
    assert!(query.planning_drafts[0].canceled_at.is_none());
}

#[test]
fn initiation_failure_matrix_leaves_no_false_initiation_root() {
    let initiate =
        |token: String, actor: &str, key: &str| super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: token,
            actor_id: actor.into(),
            idempotency_key: key.into(),
        };
    let repository = repository_at(time());
    let mut missing_draft = initiate("missing".into(), "application-user", "missing-draft");
    missing_draft.epic_planning_draft_id = EpicPlanningDraftId::new("missing-draft").unwrap();
    assert!(matches!(
        repository.initiate_epic(missing_draft),
        Err(super::super::domain::InitiateEpicError::DraftNotFound)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate(
            "missing".into(),
            "application-user",
            "missing-proposal"
        )),
        Err(super::super::domain::InitiateEpicError::ProposalMissing)
    ));
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("first"), "first"))
        .unwrap();
    assert!(matches!(
        repository.initiate_epic(initiate("stale".into(), "application-user", "stale")),
        Err(super::super::domain::InitiateEpicError::RevisionConflict)
    ));
    assert!(matches!(
        repository.initiate_epic(initiate(
            saved.revision_token.clone(),
            "caller",
            "forbidden"
        )),
        Err(super::super::domain::InitiateEpicError::Forbidden)
    ));
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").unwrap();
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-before-initiation")
        .unwrap();
    assert!(matches!(
        repository.initiate_epic(initiate(
            saved.revision_token,
            "application-user",
            "canceled"
        )),
        Err(super::super::domain::InitiateEpicError::Canceled)
    ));

    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("rollback"), "rollback-save"))
        .unwrap();
    repository.lock().unwrap().execute_batch("CREATE TRIGGER reject_initiated_sprint BEFORE INSERT ON initiated_sprints BEGIN SELECT RAISE(ABORT, 'induced rollback'); END;").unwrap();
    assert!(
        matches!(repository.initiate_epic(initiate(saved.revision_token, "application-user", "rollback")), Err(super::super::domain::InitiateEpicError::Unavailable(message)) if message.contains("induced rollback"))
    );
    let connection = repository.lock().unwrap();
    for table in [
        "epic_initiation_commands",
        "epic_initiation_results",
        "epic_initiation_events",
        "epic_initiation_provenance",
        "epic_initiation_material_snapshots",
        "epic_initiations",
        "initiated_planning_drafts",
        "initiated_sprints",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0,
            "{table} must roll back"
        );
    }
}

#[test]
fn draft_title_and_cancellation_are_durable_idempotent_and_block_further_effects() {
    let repository = repository_at(time());
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft");
    repository
        .update_planning_draft_title(&draft, SESSION_ID, Some("Separate draft title"), "title-1")
        .expect("title");
    repository
        .update_planning_draft_title(&draft, SESSION_ID, Some("Separate draft title"), "title-1")
        .expect("title retry");
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-1")
        .expect("cancel");
    repository
        .cancel_planning_draft(&draft, SESSION_ID, "cancel-1")
        .expect("cancel retry");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("denied"), "save-after-cancel")),
        Err(SaveProposalError::DraftNotFound)
    ));
    assert!(matches!(
        repository.bootstrap_managed_plan_builder(SESSION_ID),
        Err(SaveProposalError::Forbidden)
    ));
    let query = repository.native_query().expect("query");
    assert_eq!(
        query.planning_drafts[0].title.as_deref(),
        Some("Separate draft title")
    );
    assert_eq!(query.planning_drafts[0].status, "canceled");
    assert!(query.planning_drafts[0].canceled_at.is_some());
}

#[test]
fn bootstrap_is_idempotent_per_session_and_allocates_distinct_drafts() {
    let repository = repository_at(time());
    {
        let connection = repository.lock().expect("connection");
        connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES ('second-session', 'Second', 'available', '{}', ?1, ?1)", params![timestamp(time())]).expect("second session");
    }
    let first = repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .expect("first binding");
    let first_retry = repository
        .bootstrap_managed_plan_builder(SESSION_ID)
        .expect("first retry");
    let second = repository
        .bootstrap_managed_plan_builder("second-session")
        .expect("second binding");
    assert_eq!(first, first_retry);
    assert_ne!(first.0, second.0);
    assert_ne!(first.2, second.2);
}

#[test]
fn saves_typed_proposals_revises_and_retries_without_duplicate_effects() {
    let repository = Arc::new(repository_at(time()));
    let application = OrchestrationApplication::new(repository.clone());
    let first = application
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("first save");
    let duplicate = application
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("duplicate retry");
    assert_eq!(duplicate.revision_id, first.revision_id);
    assert_eq!(duplicate.revision_token, first.revision_token);
    assert!(!first.idempotent_replay);
    assert!(duplicate.idempotent_replay);
    let second = application
        .save_epic_plan_proposal(command(
            Some(first.revision_token.clone()),
            proposal("two"),
            "key-2",
        ))
        .expect("revision save");
    assert_ne!(second.revision_id, first.revision_id);
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.proposal_revisions.len(), 2);
    assert!(query
        .proposal_revisions
        .iter()
        .any(|revision| revision.proposal == proposal("one")));
    assert!(query
        .proposal_revisions
        .iter()
        .any(|revision| revision.proposal == proposal("two")));
    assert_eq!(query.recorded_proposal_events.len(), 2);
    assert_eq!(query.provenance_links.len(), 2);
}

#[test]
fn rejects_collisions_stale_malformed_and_unauthorized_requests_without_writes() {
    let repository = repository_at(time());
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("first save");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("changed"), "key-1")),
        Err(SaveProposalError::IdempotencyConflict)
    ));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("two"), "key-2")),
        Err(SaveProposalError::RevisionConflict)
    ));
    let mut forbidden = command(Some(first.revision_token.clone()), proposal("two"), "key-3");
    forbidden.agent_session_association_id =
        PlanningDraftAgentSessionAssociationId::new("wrong-association").expect("id");
    assert!(matches!(
        repository.save_epic_plan_proposal(forbidden),
        Err(SaveProposalError::Forbidden)
    ));
    let mut cross_session = command(
        Some(first.revision_token.clone()),
        proposal("cross-session"),
        "key-cross-session",
    );
    cross_session.agent_session_id = "another-session".into();
    assert!(matches!(
        repository.save_epic_plan_proposal(cross_session),
        Err(SaveProposalError::Forbidden)
    ));
    let mut malformed = command(
        Some(first.revision_token.clone()),
        PlanBuilderProposal {
            suggested_epic_name: Some(" ".into()),
            sprints: vec![],
        },
        "key-4",
    );
    malformed.proposal.suggested_epic_name = Some(" ".into());
    assert!(matches!(
        repository.save_epic_plan_proposal(malformed),
        Err(SaveProposalError::InvalidInput(_))
    ));
    let mut oversized = proposal("oversized");
    oversized.sprints[0].title = "x".repeat(241);
    assert!(matches!(
        repository.save_epic_plan_proposal(command(Some(first.revision_token), oversized, "key-5")),
        Err(SaveProposalError::InvalidInput(_))
    ));
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.proposal_revisions.len(), 1);
    assert_eq!(query.recorded_proposal_events.len(), 1);
}

#[test]
fn trusted_time_blocks_backdating_and_denies_expired_duplicate_retries() {
    let clock = Arc::new(MutableClock::new(at(2030, 1, 1)));
    let repository = repository_with_clock(clock.clone(), at(2029, 1, 1));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("backdated"), "backdate")),
        Err(SaveProposalError::Forbidden)
    ));

    let clock = Arc::new(MutableClock::new(time()));
    let repository = repository_with_clock(clock.clone(), at(2030, 1, 1));
    let first = repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("save while authorized");
    clock.set(at(2031, 1, 1));
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("one"), "key-1")),
        Err(SaveProposalError::Forbidden)
    ));
    let query = repository.native_query_at(at(2031, 1, 1)).expect("query");
    assert_eq!(query.proposal_revisions.len(), 1);
    assert_eq!(
        query.provenance_links[0].recorded_at,
        "2026-07-15T12:00:00.000Z"
    );
    assert_eq!(
        query.provenance_links[0].causal_command_id,
        first.command_id.as_str()
    );

    let repository = repository_at(time());
    repository
        .save_epic_plan_proposal(command(None, proposal("disabled"), "disabled-key"))
        .expect("save while active");
    repository
        .connection
        .lock()
        .expect("database")
        .execute(
            "UPDATE capability_profiles SET status = 'disabled' WHERE id = 'capability-profile-1'",
            [],
        )
        .expect("disable profile");
    assert!(matches!(
        repository.save_epic_plan_proposal(command(None, proposal("disabled"), "disabled-key")),
        Err(SaveProposalError::Forbidden)
    ));
}

#[test]
fn association_identity_is_independent_from_the_provider_neutral_agent_session_id() {
    let repository = repository_at(time());
    let query = repository.native_query_at(time()).expect("query");
    assert_eq!(query.agent_session_associations.len(), 1);
    let association = &query.agent_session_associations[0];
    assert_eq!(association.agent_session_association_id, ASSOCIATION_ID);
    assert_eq!(association.agent_session_id, SESSION_ID);
    assert_ne!(
        association.agent_session_association_id,
        association.agent_session_id
    );
}

#[test]
fn query_has_explicit_empty_state_and_matches_canonical_empty_fixture() {
    let json = serde_json::to_value(
        &repository_at(time())
            .native_query_at(time())
            .expect("query"),
    )
    .expect("json");
    assert_eq!(
        json,
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-empty.json"
        ))
        .expect("fixture")
    );
}

#[test]
fn populated_native_query_and_rejected_boundaries_match_rust_golden_fixtures() {
    let json = serde_json::to_value(&canonical_populated_query()).expect("json");
    assert_eq!(
        json,
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-proposal.json"
        ))
        .expect("fixture")
    );
    for (error, fixture) in [
        (
            SaveProposalError::RevisionConflict,
            include_str!("../fixtures/orchestration-native-query-v2/rejected-stale-revision.json"),
        ),
        (
            SaveProposalError::IdempotencyConflict,
            include_str!(
                "../fixtures/orchestration-native-query-v2/rejected-idempotency-conflict.json"
            ),
        ),
        (
            SaveProposalError::Forbidden,
            include_str!("../fixtures/orchestration-native-query-v2/rejected-forbidden.json"),
        ),
    ] {
        assert_eq!(
            serde_json::to_value(GoldenBoundaryDto::from(error)).expect("json"),
            serde_json::from_str::<serde_json::Value>(fixture).expect("fixture")
        );
    }
}

#[test]
fn initiated_native_query_serialization_matches_the_frontend_golden_fixture() {
    assert_eq!(
        serde_json::to_value(&canonical_initiated_query()).expect("json"),
        current_native_fixture(include_str!(
            "../fixtures/orchestration-native-query-v2/valid-initiated-epic.json"
        ))
        .expect("fixture")
    );
}

#[test]
fn restart_preserves_query_without_product_acceptance_or_initiated_identity() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("unified.sqlite");
    let connection = initialized_connection(&path);
    let clock = Arc::new(MutableClock::new(time()));
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, at(2030, 1, 1));
    repository
        .save_epic_plan_proposal(command(None, proposal("one"), "key-1"))
        .expect("save");
    let before =
        serde_json::to_string(&repository.native_query_at(time()).expect("query")).expect("json");
    drop(repository);
    let after = serde_json::to_string(
        &SqliteOrchestrationRepository::open(&path)
            .expect("reopen")
            .native_query_at(time())
            .expect("query"),
    )
    .expect("json");
    assert_eq!(before, after);
    assert!(!after.contains("accepted"));
    assert!(after.contains("\"initiatedEpics\":[]"));
    assert!(!after.contains("sprintId"));
}

#[test]
fn restart_preserves_initiated_epic_and_ordered_preparatory_sprints() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("unified.sqlite");
    let connection = crate::storage::open_active_database(&path).expect("active database");
    seed_session(&connection);
    let clock = Arc::new(MutableClock::new(time()));
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, at(2030, 1, 1));
    let mut submitted = proposal("restart");
    submitted.sprints.push(ProposedSprint {
        title: "Second preparatory Sprint".into(),
        intended_movement: "Keep the persisted proposal order.".into(),
        concern_summaries: vec!["No execution was initiated.".into()],
    });
    let saved = repository
        .save_epic_plan_proposal(command(None, submitted, "restart-save"))
        .expect("save proposal");
    let initiated = repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "restart-initiation".into(),
        })
        .expect("initiate");
    let before = repository.native_query_at(time()).expect("query");
    drop(repository);

    drop(crate::storage::open_active_database(&path).expect("reopen active database"));
    let after = SqliteOrchestrationRepository::open(&path)
        .expect("reopen repository")
        .native_query_at(time())
        .expect("query");
    assert_eq!(
        serde_json::to_value(&after).expect("json"),
        serde_json::to_value(&before).expect("json")
    );
    assert_eq!(after.planning_drafts[0].status, "initiated");
    assert_eq!(after.initiated_epics[0].epic_id, initiated.epic_id.as_str());
    assert_eq!(after.material_snapshots.len(), 1);
    assert_eq!(after.initiated_sprints.len(), 2);
    assert_eq!(after.initiated_sprints[0].ordinal, 0);
    assert_eq!(after.initiated_sprints[1].ordinal, 1);
    assert_eq!(after.initiated_sprints[0].title, "Sprint restart");
    assert_eq!(
        after.initiated_sprints[1].title,
        "Second preparatory Sprint"
    );
}

fn repository_at(now: chrono::DateTime<Utc>) -> SqliteOrchestrationRepository {
    repository_with_clock(Arc::new(MutableClock::new(now)), at(2030, 1, 1))
}

struct RealGitRepository {
    root: std::path::PathBuf,
    _temp: tempfile::TempDir,
    baseline: String,
    current: String,
}

fn real_file_review_repository() -> RealGitRepository {
    let temp = tempfile::tempdir().expect("temp repo");
    let root = temp.path().to_path_buf();
    init_git(&root);
    write_git_file(&root, "modified.txt", b"before\nline\n");
    write_git_file(&root, "deleted.md", b"# Gone\n\nBody\n");
    write_git_file(&root, "docs/old-name.md", b"# Retained\n");
    write_git_file(&root, "binary.bin", &[0, 1, 2]);
    write_git_file(&root, "unsupported.dat", &[0xff, 0xfe]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "baseline"]);
    let baseline = git_text(&root, &["rev-parse", "HEAD"]);

    write_git_file(&root, "modified.txt", b"after\nline\n");
    fs::remove_file(root.join("deleted.md")).expect("delete file");
    git(&root, &["mv", "docs/old-name.md", "docs/new-name.md"]);
    write_git_file(&root, "added.md", b"# Added\n\nReview text.\n");
    write_git_file(&root, "binary.bin", &[0, 9, 2]);
    write_git_file(&root, "unsupported.dat", &[0xff, 0xfd]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "current"]);
    let current = git_text(&root, &["rev-parse", "HEAD"]);
    RealGitRepository {
        root,
        _temp: temp,
        baseline,
        current,
    }
}

fn initiated_repository_with_capture(
    capture_authorization_id: &str,
    git_repository: &RealGitRepository,
) -> SqliteOrchestrationRepository {
    let repository = repository_at(time());
    let saved = repository
        .save_epic_plan_proposal(command(None, proposal("git review"), "git-review-save"))
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").unwrap(),
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "git-review-init".into(),
        })
        .unwrap();
    let query = repository.native_query().unwrap();
    let epic = &query.initiated_epics[0];
    repository
        .store_file_review_git_capture_authorization(FileReviewGitCaptureAuthorizationWrite {
            capture_authorization_id: capture_authorization_id.into(),
            idempotency_key: format!("{capture_authorization_id}-authorization"),
            epic_id: epic.epic_id.clone(),
            sprint_id: query.initiated_sprints[0].sprint_id.clone(),
            provenance_id: epic.provenance_id.clone(),
            repository_id: "authorized-repository".into(),
            repository_root: canonical_root(&git_repository.root),
            worktree_id: "authorized-worktree".into(),
            worktree_root: canonical_root(&git_repository.root),
            baseline_object_id: git_repository.baseline.clone(),
            current_object_id: git_repository.current.clone(),
        })
        .unwrap();
    repository
}

fn initiate_second_epic(repository: &SqliteOrchestrationRepository) -> String {
    let existing_provenance = repository.native_query().unwrap().initiated_epics[0]
        .provenance_id
        .clone();
    let draft = EpicPlanningDraftId::new("epic-planning-draft-git-other").unwrap();
    let profile = CapabilityProfileId::new("capability-profile-git-other").unwrap();
    let association = PlanningDraftAgentSessionAssociationId::new(
        "planning-draft-agent-session-association-git-other",
    )
    .unwrap();
    repository.create_planning_draft(&draft, time()).unwrap();
    repository
        .create_capability_profile(&profile, "active", time())
        .unwrap();
    repository
        .associate_agent_session(&association, &draft, SESSION_ID, ACTOR_ID, time())
        .unwrap();
    repository
        .assign_profile(&draft, &profile, &association, at(2030, 1, 1), time())
        .unwrap();
    let saved = repository
        .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
            epic_planning_draft_id: draft.clone(),
            capability_profile_id: profile,
            agent_session_association_id: association,
            agent_session_id: SESSION_ID.into(),
            actor_id: ACTOR_ID.into(),
            expected_revision: None,
            proposal: proposal("git other"),
            idempotency_key: "git-other-save".into(),
        })
        .unwrap();
    repository
        .initiate_epic(super::super::domain::InitiateEpicCommand {
            epic_planning_draft_id: draft,
            expected_revision_token: saved.revision_token,
            actor_id: "application-user".into(),
            idempotency_key: "git-other-init".into(),
        })
        .unwrap();
    repository
        .native_query()
        .unwrap()
        .initiated_epics
        .into_iter()
        .find(|epic| epic.provenance_id != existing_provenance)
        .expect("second epic")
        .provenance_id
}

fn canonical_root(path: &Path) -> String {
    fs::canonicalize(path)
        .expect("canonical root")
        .to_string_lossy()
        .into_owned()
}

fn init_git(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.name", "File Review Test"]);
    git(
        root,
        &["config", "user.email", "file-review@example.invalid"],
    );
    git(root, &["config", "commit.gpgsign", "false"]);
}

fn write_git_file(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories");
    }
    fs::write(path, bytes).expect("write git file");
}

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn git_text(root: &Path, args: &[&str]) -> String {
    String::from_utf8(git(root, args))
        .expect("utf8 git output")
        .trim()
        .to_string()
}
fn repository_with_clock(
    clock: Arc<MutableClock>,
    expiry: chrono::DateTime<Utc>,
) -> SqliteOrchestrationRepository {
    let connection = initialized_connection_in_memory();
    let repository =
        SqliteOrchestrationRepository::new_with_clock(connection, clock).expect("repository");
    seed(&repository, expiry);
    repository
}
fn initialized_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("open");
    initialize_connection(&connection);
    connection
}
fn initialized_connection_in_memory() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    initialize_connection(&connection);
    connection
}
fn initialize_connection(connection: &Connection) {
    crate::storage::configure_sqlite_connection(connection).expect("policy");
    connection
        .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
        .expect("session schema");
    connection
        .execute_batch(ORCHESTRATION_SCHEMA)
        .expect("orchestration schema");
    connection
        .execute_batch(ORCHESTRATION_INITIATION_SCHEMA)
        .expect("initiation schema");
    connection
        .execute_batch(PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA)
        .expect("context delivery schema");
    connection
        .execute_batch(FILE_REVIEW_FACTS_SCHEMA)
        .expect("File Review facts schema");
    seed_session(connection);
}
fn seed(repository: &SqliteOrchestrationRepository, expiry: chrono::DateTime<Utc>) {
    let draft = EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft");
    let profile = CapabilityProfileId::new("capability-profile-1").expect("profile");
    let association =
        PlanningDraftAgentSessionAssociationId::new(ASSOCIATION_ID).expect("association");
    repository
        .create_planning_draft(&draft, time())
        .expect("draft");
    repository
        .create_capability_profile(&profile, "active", time())
        .expect("profile");
    repository
        .associate_agent_session(&association, &draft, SESSION_ID, ACTOR_ID, time())
        .expect("association");
    repository
        .assign_profile(&draft, &profile, &association, expiry, time())
        .expect("assignment");
}
fn seed_session(connection: &Connection) {
    connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES (?1, 'Plan Builder session', 'available', '{}', ?2, ?2)", params![SESSION_ID, timestamp(time())]).expect("session");
}
fn command(
    expected_revision: Option<String>,
    proposal: PlanBuilderProposal,
    key: &str,
) -> SaveEpicPlanProposalCommand {
    SaveEpicPlanProposalCommand {
        epic_planning_draft_id: EpicPlanningDraftId::new("epic-planning-draft-1").expect("draft"),
        capability_profile_id: CapabilityProfileId::new("capability-profile-1").expect("profile"),
        agent_session_association_id: PlanningDraftAgentSessionAssociationId::new(ASSOCIATION_ID)
            .expect("association"),
        agent_session_id: SESSION_ID.into(),
        actor_id: ACTOR_ID.into(),
        expected_revision,
        proposal,
        idempotency_key: key.into(),
    }
}
fn proposal(suffix: &str) -> PlanBuilderProposal {
    PlanBuilderProposal {
        suggested_epic_name: Some(format!("Suggested Epic {suffix}")),
        sprints: vec![ProposedSprint {
            title: format!("Sprint {suffix}"),
            intended_movement: format!("Move {suffix} forward."),
            concern_summaries: vec![format!("Concern {suffix}.")],
        }],
    }
}
fn time() -> chrono::DateTime<Utc> {
    at(2026, 7, 15)
}
fn at(year: i32, month: u32, day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0)
        .single()
        .expect("time")
}

struct MutableClock(Mutex<chrono::DateTime<Utc>>);
impl MutableClock {
    fn new(now: chrono::DateTime<Utc>) -> Self {
        Self(Mutex::new(now))
    }
    fn set(&self, now: chrono::DateTime<Utc>) {
        *self.0.lock().expect("clock") = now;
    }
}
impl OrchestrationClock for MutableClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        *self.0.lock().expect("clock")
    }
}

fn canonical_populated_query() -> NativeQueryV2 {
    NativeQueryV2 {
        contract_version: NATIVE_QUERY_VERSION,
        generated_at: "2026-07-15T12:00:00.000Z".into(),
        planning_drafts: vec![PlanningDraftDto {
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            title: None,
            status: "active".into(),
            created_at: "2026-07-15T12:00:00.000Z".into(),
            updated_at: "2026-07-15T12:00:00.000Z".into(),
            canceled_at: None,
            current_proposal: CurrentProposalDto::Available {
                proposal_revision_id: "proposal-revision-fixture".into(),
            },
        }],
        agent_session_associations: vec![AgentSessionAssociationDto {
            agent_session_association_id: "association-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            agent_session_id: "agent-session-fixture".into(),
            actor_id: "managed-plan-builder".into(),
            associated_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        proposal_revisions: vec![ProposalRevisionDto {
            proposal_revision_id: "proposal-revision-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            parent_proposal_revision_id: None,
            revision_token: "proposal-token-fixture".into(),
            proposal: proposal("fixture"),
            command_id: "proposal-command-fixture".into(),
            provenance_id: "provenance-fixture".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        recorded_proposal_events: vec![RecordedProposalEventDto {
            proposal_event_id: "proposal-event-fixture".into(),
            epic_planning_draft_id: "epic-planning-draft-fixture".into(),
            proposal_revision_id: "proposal-revision-fixture".into(),
            command_id: "proposal-command-fixture".into(),
            provenance_id: "provenance-fixture".into(),
            event_kind: "proposal_saved".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
        }],
        provenance_links: vec![ProvenanceLinkDto {
            provenance_id: "provenance-fixture".into(),
            source_kind: "managed_plan_builder".into(),
            recorded_at: "2026-07-15T12:00:00.000Z".into(),
            actor_id: "managed-plan-builder".into(),
            agent_session_association_id: "association-fixture".into(),
            capability_profile_id: "capability-profile-fixture".into(),
            causal_command_id: "proposal-command-fixture".into(),
            causal_result_id: "proposal-result-fixture".into(),
        }],
        initiation_commands: vec![],
        initiation_results: vec![],
        initiation_events: vec![],
        initiation_provenance: vec![],
        material_snapshots: vec![],
        initiated_epics: vec![],
        initiated_sprints: vec![],
        file_review_documents: vec![],
        work_unit_materializations: vec![],
        work_units: vec![],
        work_unit_relationships: vec![],
    }
}

fn current_native_fixture(value: &str) -> Result<serde_json::Value, serde_json::Error> {
    let mut fixture = serde_json::from_str::<serde_json::Value>(value)?;
    fixture
        .as_object_mut()
        .unwrap()
        .insert("fileReviewDocuments".into(), serde_json::json!([]));
    fixture
        .as_object_mut()
        .unwrap()
        .insert("workUnitMaterializations".into(), serde_json::json!([]));
    fixture
        .as_object_mut()
        .unwrap()
        .insert("workUnits".into(), serde_json::json!([]));
    fixture
        .as_object_mut()
        .unwrap()
        .insert("workUnitRelationships".into(), serde_json::json!([]));
    Ok(fixture)
}

fn canonical_initiated_query() -> NativeQueryV2 {
    let mut query = canonical_populated_query();
    let mut proposal = proposal("fixture");
    proposal.sprints.push(ProposedSprint {
        title: "Second Sprint fixture".into(),
        intended_movement: "Move second fixture forward.".into(),
        concern_summaries: vec!["Second concern fixture.".into()],
    });
    query.planning_drafts[0].status = "initiated".into();
    query.proposal_revisions[0].proposal = proposal.clone();
    query.initiation_commands = vec![InitiationCommandDto {
        command_id: "init-command-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        expected_revision_token: "proposal-token-fixture".into(),
        actor_id: "application-user".into(),
        idempotency_key: "initiate:epic-planning-draft-fixture:proposal-revision-fixture".into(),
        payload_fingerprint: "epic-planning-draft-fixture:proposal-token-fixture:application-user"
            .into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_results = vec![InitiationResultDto {
        result_id: "init-result-fixture".into(),
        command_id: "init-command-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_events = vec![InitiationEventDto {
        event_id: "init-event-fixture".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiation_provenance = vec![InitiationProvenanceDto {
        provenance_id: "init-provenance-fixture".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        event_id: "init-event-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.material_snapshots = vec![MaterialSnapshotDto {
        material_snapshot_id: "snapshot-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        proposal_revision_id: "proposal-revision-fixture".into(),
        version: 1,
        proposal,
        content_hash: "24a39e80f2f30ceb15c9488d9e9e48f6cf462b44ee1b69e6133342495d94dae8".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
    }];
    query.initiated_epics = vec![InitiatedEpicDto {
        initiation_id: "initiation-fixture".into(),
        epic_planning_draft_id: "epic-planning-draft-fixture".into(),
        proposal_revision_id: "proposal-revision-fixture".into(),
        material_snapshot_id: "snapshot-fixture".into(),
        epic_id: "epic-fixture".into(),
        recorded_at: "2026-07-15T12:00:00.000Z".into(),
        command_id: "init-command-fixture".into(),
        result_id: "init-result-fixture".into(),
        event_id: "init-event-fixture".into(),
        provenance_id: "init-provenance-fixture".into(),
    }];
    query.initiated_sprints = vec![
        InitiatedSprintDto {
            sprint_id: "sprint-fixture".into(),
            epic_id: "epic-fixture".into(),
            ordinal: 0,
            title: "Sprint fixture".into(),
            intended_movement: "Move fixture forward.".into(),
            concern_summaries: vec!["Concern fixture.".into()],
            sprint_plan_id: "sprint-plan-fixture".into(),
            sprint_plan_revision_id: "sprint-plan-revision-fixture".into(),
        },
        InitiatedSprintDto {
            sprint_id: "sprint-fixture-2".into(),
            epic_id: "epic-fixture".into(),
            ordinal: 1,
            title: "Second Sprint fixture".into(),
            intended_movement: "Move second fixture forward.".into(),
            concern_summaries: vec!["Second concern fixture.".into()],
            sprint_plan_id: "sprint-plan-fixture-2".into(),
            sprint_plan_revision_id: "sprint-plan-revision-fixture-2".into(),
        },
    ];
    query
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenBoundaryDto {
    code: &'static str,
    is_error: bool,
    state_authority: &'static str,
}
impl From<SaveProposalError> for GoldenBoundaryDto {
    fn from(error: SaveProposalError) -> Self {
        let code = match error {
            SaveProposalError::RevisionConflict => "revision_conflict",
            SaveProposalError::IdempotencyConflict => "idempotency_conflict",
            SaveProposalError::Forbidden => "forbidden",
            _ => unreachable!("fixture error"),
        };
        Self {
            code,
            is_error: true,
            state_authority: "none",
        }
    }
}

#[test]
fn implementer_activation_projection_serializes_every_persisted_fact_at_its_named_column() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE work_unit_implementer_activations (
           work_unit_id TEXT PRIMARY KEY, handler_attempt_id TEXT NOT NULL,
           handler_invocation_id TEXT NOT NULL, attempt_id TEXT NOT NULL,
           implementer_session_id TEXT NOT NULL, implementer_invocation_id TEXT NOT NULL,
           implementer_harness_revision_id TEXT NOT NULL,
           implementer_harness_configuration_digest TEXT NOT NULL,
           implementer_harness_repository_commit_ref TEXT NOT NULL, requested_at TEXT NOT NULL,
           authorized_at TEXT, execution_support_granted_at TEXT, isolated_worktree_ready_at TEXT,
           implementer_session_created_at TEXT, implementer_invocation_prepared_at TEXT,
           implementer_harness_bound_at TEXT, launch_requested_at TEXT, launch_accepted_at TEXT,
           provider_activation_observed_at TEXT, implementer_ready_at TEXT, failure_reason TEXT
         );
         INSERT INTO work_unit_implementer_activations VALUES
           ('unit','handler-attempt','handler-action','shared-attempt','implementer-session',
            'implementer-invocation','implementer-revision','digest','commit','requested',
            'authorized','support','worktree','session-created','prepared','bound','launch-requested',
            'launch-accepted','provider-observed','ready','precise-failure');",
    ).unwrap();
    let activations = activation_rows(
        &connection, "work_unit_implementer_activations",
        "attempt_id,handler_invocation_id,implementer_session_id,implementer_invocation_id,implementer_harness_revision_id,implementer_harness_configuration_digest,implementer_harness_repository_commit_ref,requested_at,authorized_at,execution_support_granted_at,isolated_worktree_ready_at,implementer_session_created_at,implementer_invocation_prepared_at,implementer_harness_bound_at,launch_requested_at,launch_accepted_at,provider_activation_observed_at,implementer_ready_at,failure_reason",
        map_implementer_activation,
    ).unwrap();
    let value = serde_json::to_value(activations.get("unit").unwrap()).unwrap();
    assert_eq!(value["attemptId"], "shared-attempt");
    assert_eq!(value["handlerActionInvocationId"], "handler-action");
    assert_eq!(value["implementerSessionId"], "implementer-session");
    assert_eq!(value["implementerInvocationId"], "implementer-invocation");
    assert_eq!(value["implementerHarnessRevisionId"], "implementer-revision");
    assert_eq!(value["implementerHarnessConfigurationDigest"], "digest");
    assert_eq!(value["implementerHarnessRepositoryCommitRef"], "commit");
    for (key, expected) in [
        ("requestedAt", "requested"), ("authorizedAt", "authorized"),
        ("executionSupportGrantedAt", "support"), ("isolatedWorktreeReadyAt", "worktree"),
        ("implementerSessionCreatedAt", "session-created"),
        ("implementerInvocationPreparedAt", "prepared"), ("implementerHarnessBoundAt", "bound"),
        ("launchRequestedAt", "launch-requested"), ("launchAcceptedAt", "launch-accepted"),
        ("providerActivationObservedAt", "provider-observed"), ("implementerReadyAt", "ready"),
        ("failureReason", "precise-failure"),
    ] { assert_eq!(value[key], expected); }
}

#[test]
fn work_unit_activation_projection_fails_closed_for_foreign_or_incoherent_state() {
    let valid = valid_work_unit_activation_projection();
    assert!(validate_work_unit_activation_projection(&valid).is_ok());

    let mut foreign_original = valid_work_unit_activation_projection();
    foreign_original.action_continuation.as_mut().unwrap().original_handler_invocation_id =
        "foreign-original".into();
    assert!(validate_work_unit_activation_projection(&foreign_original).is_err());

    let mut missing_prerequisite = valid_work_unit_activation_projection();
    missing_prerequisite.handler_activation.as_mut().unwrap().isolated_worktree_ready_at = None;
    assert!(validate_work_unit_activation_projection(&missing_prerequisite).is_err());

    let mut stale_block = valid_work_unit_activation_projection();
    stale_block.action_continuation.as_mut().unwrap().blocked_reason = Some("stale".into());
    assert!(validate_work_unit_activation_projection(&stale_block).is_err());

    let mut failed_and_ready = valid_work_unit_activation_projection();
    failed_and_ready.implementer_activation.as_mut().unwrap().failure_reason = Some("failed".into());
    assert!(validate_work_unit_activation_projection(&failed_and_ready).is_err());
}

#[test]
fn implementer_outcome_projection_serializes_authoritative_claim_evidence_and_readiness_facts() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    create_implementer_outcome_projection_table(&connection);
    let attempt = "attempt";
    let reporting = projection_stable_id("work-unit-implementer-reporting-invocation", attempt);
    let payload = r#"{"outcome":"review_pending","summary":"Implemented the bounded change.","validationStatement":"Focused checks passed."}"#;
    let submission_fingerprint = projection_stable_id("implementer-outcome", payload);
    connection.execute(
        "INSERT INTO work_unit_implementer_outcomes VALUES (
          'unit',?1,'implementer-session','implementer-invocation',?2,
          'reporting-revision','reporting-digest','reporting-commit',
          '2026-08-04T00:00:00Z','2026-08-04T00:00:01Z','2026-08-04T00:00:02Z',
          '2026-08-04T00:00:03Z','2026-08-04T00:00:04Z','2026-08-04T00:00:05Z',
          'Implemented the bounded change.','review_pending','Focused checks passed.',?3,?4,
          '2026-08-04T00:00:06Z','2026-08-04T00:00:06Z','valid',?5,'comparison-fingerprint',?6,
          '2026-08-04T00:00:07Z','2026-08-04T00:00:08Z',?2,
          '2026-08-04T00:00:09Z','completed','2026-08-04T00:00:10Z',
          '2026-08-04T00:00:11Z',NULL)",
        params![
            attempt,
            reporting,
            payload,
            submission_fingerprint,
            r#"[{"evidenceRef":"evidence-1","displayName":"src/lib.rs","changeKind":"modified"}]"#,
            r#"[{"evidenceRef":"evidence-1","contentFingerprint":"content-fingerprint"}]"#,
        ],
    ).unwrap();

    let outcomes = implementer_outcome_rows(&connection).unwrap();
    let value = serde_json::to_value(outcomes.get("unit").unwrap()).unwrap();
    assert_eq!(value["submittedOutcome"]["variant"], "review_pending");
    assert_eq!(value["submittedOutcome"]["summaryClaim"], "Implemented the bounded change.");
    assert_eq!(value["submittedOutcome"]["validationStatementClaim"], "Focused checks passed.");
    assert_eq!(value["evidence"]["changedFiles"][0]["evidenceRef"], "evidence-1");
    assert_eq!(value["evidence"]["changedFiles"][0]["contentFingerprint"], "content-fingerprint");
    assert_eq!(value["terminalLifecycle"]["status"], "completed");
    assert_eq!(value["applicationAcceptedAt"], "2026-08-04T00:00:10Z");
    assert_eq!(value["handlerReviewReadyAt"], "2026-08-04T00:00:11Z");
}

#[test]
fn implementer_outcome_projection_rejects_partial_bundles_and_incoherent_authority() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    create_implementer_outcome_projection_table(&connection);
    let reporting = projection_stable_id("work-unit-implementer-reporting-invocation", "attempt");
    connection.execute(
        "INSERT INTO work_unit_implementer_outcomes (
          work_unit_id,attempt_id,implementer_session_id,implementer_invocation_id,
          reporting_invocation_id,reporting_harness_revision_id,
          reporting_harness_configuration_digest,reporting_harness_repository_commit_ref,
          reporting_requested_at,submitted_summary
        ) VALUES ('unit','attempt','implementer-session','implementer-invocation',?1,
          'reporting-revision','reporting-digest','reporting-commit','2026-08-04T00:00:00Z','partial')",
        [&reporting],
    ).unwrap();
    assert!(implementer_outcome_rows(&connection).is_err());

    let valid = valid_work_unit_outcome_projection();
    assert!(validate_work_unit_activation_projection(&valid).is_ok());

    let mut foreign_session = valid_work_unit_outcome_projection();
    foreign_session.implementer_outcome.as_mut().unwrap().implementer_session_id = "foreign".into();
    assert!(validate_work_unit_activation_projection(&foreign_session).is_err());

    let mut reused_invocation = valid_work_unit_outcome_projection();
    reused_invocation.implementer_outcome.as_mut().unwrap().reporting_invocation_id =
        "implementer-invocation".into();
    assert!(validate_work_unit_activation_projection(&reused_invocation).is_err());

    let mut accepted_failed = valid_work_unit_outcome_projection();
    accepted_failed.implementer_outcome.as_mut().unwrap().terminal_lifecycle.as_mut().unwrap().status =
        WorkUnitImplementerLifecycleStatusDto::Failed;
    assert!(validate_work_unit_activation_projection(&accepted_failed).is_err());

    let mut ready_without_acceptance = valid_work_unit_outcome_projection();
    ready_without_acceptance.implementer_outcome.as_mut().unwrap().application_accepted_at = None;
    assert!(validate_work_unit_activation_projection(&ready_without_acceptance).is_err());

    let mut out_of_order = valid_work_unit_outcome_projection();
    out_of_order.implementer_outcome.as_mut().unwrap().reporting_prepared_at =
        Some("2026-08-03T23:59:59Z".into());
    assert!(validate_work_unit_activation_projection(&out_of_order).is_err());
}

fn create_implementer_outcome_projection_table(connection: &rusqlite::Connection) {
    connection.execute_batch(
        "CREATE TABLE work_unit_implementer_outcomes (
          work_unit_id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL,
          implementer_session_id TEXT NOT NULL, implementer_invocation_id TEXT NOT NULL,
          reporting_invocation_id TEXT NOT NULL, reporting_harness_revision_id TEXT NOT NULL,
          reporting_harness_configuration_digest TEXT NOT NULL,
          reporting_harness_repository_commit_ref TEXT NOT NULL, reporting_requested_at TEXT NOT NULL,
          reporting_prepared_at TEXT, reporting_harness_bound_at TEXT,
          reporting_launch_requested_at TEXT, reporting_launch_accepted_at TEXT, reporting_ready_at TEXT,
          submitted_summary TEXT, outcome_variant TEXT, submitted_validation_statement TEXT,
          semantic_payload_json TEXT, submission_fingerprint TEXT, submitted_at TEXT,
          validation_at TEXT, validation_result TEXT, evidence_manifest_json TEXT,
          comparison_fingerprint TEXT, evidence_content_fingerprints_json TEXT, evidence_ready_at TEXT,
          semantic_completed_at TEXT, semantic_completion_invocation_id TEXT,
          lifecycle_observed_at TEXT, lifecycle_status TEXT, application_accepted_at TEXT,
          handler_review_ready_at TEXT, failure_reason TEXT
        );"
    ).unwrap();
}

fn valid_work_unit_outcome_projection() -> WorkUnitDto {
    let mut work_unit = valid_work_unit_activation_projection();
    let reporting_invocation_id =
        projection_stable_id("work-unit-implementer-reporting-invocation", "attempt");
    work_unit.implementer_outcome = Some(WorkUnitImplementerOutcomeDto {
        attempt_id: "attempt".into(),
        implementer_session_id: "implementer-session".into(),
        original_implementer_invocation_id: "implementer-invocation".into(),
        reporting_invocation_id: reporting_invocation_id.clone(),
        reporting_harness_revision_id: "reporting-revision".into(),
        reporting_harness_configuration_digest: "reporting-digest".into(),
        reporting_harness_repository_commit_ref: "reporting-commit".into(),
        reporting_requested_at: "2026-08-04T00:00:00Z".into(),
        reporting_prepared_at: Some("2026-08-04T00:00:01Z".into()),
        reporting_harness_bound_at: Some("2026-08-04T00:00:02Z".into()),
        reporting_launch_requested_at: Some("2026-08-04T00:00:03Z".into()),
        reporting_launch_accepted_at: Some("2026-08-04T00:00:04Z".into()),
        reporting_ready_at: Some("2026-08-04T00:00:05Z".into()),
        submitted_outcome: Some(WorkUnitImplementerSubmissionDto {
            variant: ImplementationOutcomeVariantDto::ReviewPending,
            summary_claim: "Implemented the bounded change.".into(),
            validation_statement_claim: "Focused checks passed.".into(),
            semantic_payload_fingerprint: "payload-fingerprint".into(),
            submitted_at: "2026-08-04T00:00:06Z".into(),
            validation_at: "2026-08-04T00:00:06Z".into(),
            validation_result: "valid",
        }),
        evidence: Some(WorkUnitImplementerEvidenceDto {
            changed_files: vec![WorkUnitImplementerEvidenceFileDto {
                evidence_ref: "evidence-1".into(),
                display_name: "src/lib.rs".into(),
                change_kind: ImplementationEvidenceChangeKindDto::Modified,
                content_fingerprint: "content-fingerprint".into(),
            }],
            comparison_fingerprint: "comparison-fingerprint".into(),
            ready_at: "2026-08-04T00:00:07Z".into(),
        }),
        semantic_completion: Some(WorkUnitImplementerSemanticCompletionDto {
            invocation_id: reporting_invocation_id,
            completed_at: "2026-08-04T00:00:08Z".into(),
        }),
        terminal_lifecycle: Some(WorkUnitImplementerTerminalLifecycleDto {
            status: WorkUnitImplementerLifecycleStatusDto::Completed,
            observed_at: "2026-08-04T00:00:09Z".into(),
        }),
        application_accepted_at: Some("2026-08-04T00:00:10Z".into()),
        handler_review_ready_at: Some("2026-08-04T00:00:11Z".into()),
        failure_reason: None,
    });
    work_unit
}

#[test]
fn handler_review_projection_preserves_judgment_decision_and_later_workflow_boundary() {
    let mut work_unit = valid_work_unit_outcome_projection();
    work_unit.handler_review = Some(WorkUnitHandlerReviewDto {
        attempt_id: "attempt".into(),
        reporting_invocation_id: projection_stable_id("work-unit-implementer-reporting-invocation", "attempt"),
        handler_session_id: "handler-session".into(),
        original_handler_invocation_id: "handler-original".into(),
        action_handler_invocation_id: "handler-action".into(),
        review_invocation_id: projection_stable_id("work-unit-handler-review-invocation", "attempt"),
        review_harness_revision_id: "review-revision".into(),
        review_harness_configuration_digest: "review-digest".into(),
        review_harness_repository_commit_ref: "review-commit".into(),
        delivery_requested_at: "2026-08-04T00:00:12Z".into(),
        delivery_persisted_at: Some("2026-08-04T00:00:12Z".into()),
        harness_bound_at: Some("2026-08-04T00:00:13Z".into()),
        launch_requested_at: Some("2026-08-04T00:00:14Z".into()),
        launch_accepted_at: Some("2026-08-04T00:00:15Z".into()),
        review_ready_at: Some("2026-08-04T00:00:16Z".into()),
        delivered: WorkUnitHandlerReviewEvidenceDto {
            summary_claim: "Implemented the bounded change.".into(),
            validation_statement_claim: "Focused checks passed.".into(),
            changed_files: vec![WorkUnitHandlerReviewEvidenceFileDto {
                evidence_ref: "evidence-1".into(),
                display_name: "src/lib.rs".into(),
                change_kind: ImplementationEvidenceChangeKindDto::Modified,
                content_fingerprint: "content-fingerprint".into(),
            }],
            comparison_fingerprint: "comparison-fingerprint".into(),
            delivered_payload_fingerprint: "delivery-fingerprint".into(),
        },
        semantic_judgment: Some(WorkUnitHandlerReviewJudgmentDto {
            variant: WorkUnitHandlerReviewJudgmentVariantDto::Accept,
            reason: None,
            fingerprint: "judgment-fingerprint".into(),
            recorded_at: "2026-08-04T00:00:17Z".into(),
        }),
        lifecycle: Some(WorkUnitHandlerReviewLifecycleDto {
            status: WorkUnitHandlerReviewLifecycleStatusDto::Completed,
            observed_at: "2026-08-04T00:00:18Z".into(),
        }),
        conflict: None,
    });
    work_unit.handler_decision = Some(WorkUnitHandlerDecisionDto {
        review_invocation_id: projection_stable_id("work-unit-handler-review-invocation", "attempt"),
        variant: WorkUnitHandlerDecisionVariantDto::Accepted,
        fingerprint: "decision-fingerprint".into(),
        return_reason: None,
        recorded_at: "2026-08-04T00:00:19Z".into(),
        implementation_accepted_at: Some("2026-08-04T00:00:19Z".into()),
        implementation_returned_at: None,
        retry_required_at: None,
        settlement_ready_at: None,
    });
    validate_work_unit_activation_projection(&work_unit).expect("accepted review projection");

    work_unit.handler_review.as_mut().unwrap().lifecycle = Some(WorkUnitHandlerReviewLifecycleDto {
        status: WorkUnitHandlerReviewLifecycleStatusDto::Failed,
        observed_at: "2026-08-04T00:00:18Z".into(),
    });
    assert!(validate_work_unit_activation_projection(&work_unit)
        .expect_err("decision without Completed lifecycle")
        .contains("Completed review judgment"));
}

fn valid_work_unit_activation_projection() -> WorkUnitDto {
    let timestamp = || Some("2026-08-03T00:00:00Z".to_string());
    WorkUnitDto {
        work_unit_id: "unit".into(),
        materialization_id: "materialization".into(),
        work_slice_id: "slice".into(),
        accepted_revision_id: "revision".into(),
        lane_ordinal: 0,
        lane_title: "Lane".into(),
        specification: "Specification".into(),
        handler_activation: Some(WorkUnitHandlerActivationDto {
            attempt_id: "attempt".into(),
            handler_session_id: Some("handler-session".into()),
            handler_invocation_id: Some("handler-original".into()),
            handler_harness_revision_id: Some("handler-revision".into()),
            handler_harness_configuration_digest: Some("handler-digest".into()),
            handler_harness_repository_commit_ref: Some("handler-commit".into()),
            eligibility_state: Some("eligible".into()),
            blocked_reason: None,
            requested_at: timestamp(),
            authorized_at: timestamp(),
            attempt_created_at: timestamp(),
            execution_support_granted_at: timestamp(),
            isolated_worktree_ready_at: timestamp(),
            handler_session_created_at: timestamp(),
            handler_invocation_prepared_at: timestamp(),
            handler_harness_bound_at: timestamp(),
            launch_requested_at: timestamp(),
            launch_accepted_at: timestamp(),
            provider_activation_observed_at: timestamp(),
            handler_ready_at: timestamp(),
        }),
        action_continuation: Some(WorkUnitHandlerActionContinuationDto {
            attempt_id: "attempt".into(),
            handler_session_id: "handler-session".into(),
            original_handler_invocation_id: "handler-original".into(),
            action_invocation_id: "handler-action".into(),
            action_harness_revision_id: "action-revision".into(),
            action_harness_configuration_digest: "action-digest".into(),
            action_harness_repository_commit_ref: "action-commit".into(),
            requested_at: timestamp().unwrap(),
            authorized_at: timestamp(),
            invocation_prepared_at: timestamp(),
            harness_bound_at: timestamp(),
            launch_requested_at: timestamp(),
            launch_accepted_at: timestamp(),
            provider_activation_observed_at: timestamp(),
            action_ready_at: timestamp(),
            blocked_reason: None,
            failure_reason: None,
        }),
        implementer_activation: Some(WorkUnitImplementerActivationDto {
            attempt_id: "attempt".into(),
            handler_action_invocation_id: "handler-action".into(),
            implementer_session_id: "implementer-session".into(),
            implementer_invocation_id: "implementer-invocation".into(),
            implementer_harness_revision_id: "implementer-revision".into(),
            implementer_harness_configuration_digest: "implementer-digest".into(),
            implementer_harness_repository_commit_ref: "implementer-commit".into(),
            requested_at: timestamp().unwrap(),
            authorized_at: timestamp(),
            execution_support_granted_at: timestamp(),
            isolated_worktree_ready_at: timestamp(),
            implementer_session_created_at: timestamp(),
            implementer_invocation_prepared_at: timestamp(),
            implementer_harness_bound_at: timestamp(),
            launch_requested_at: timestamp(),
            launch_accepted_at: timestamp(),
            provider_activation_observed_at: timestamp(),
            implementer_ready_at: timestamp(),
            failure_reason: None,
        }),
        implementer_outcome: None,
        handler_review: None,
        handler_decision: None,
        integration: None,
    }
}

#[test]
fn productive_integration_projection_is_absent_without_owned_schema() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    assert!(productive_integration_rows(&connection, &[], &[]).unwrap().is_empty());
}

#[test]
fn productive_integration_projection_preserves_authorized_and_progressive_stages() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    let authorized = productive_integration_rows(&connection, &units, &relationships).unwrap();
    assert_eq!(authorized["unit"].requested_at, "2026-08-04T00:00:20Z");
    assert_eq!(authorized["unit"].authorized_at, "2026-08-04T00:00:20Z");
    assert!(authorized["unit"].progress.is_none());
    assert!(authorized["unit"].success.is_none());

    connection.execute_batch(
        "UPDATE accepted_work_unit_integrations SET stage='object_created',integration_commit_id='private-integration-commit',integration_tree_id='private-integration-tree',object_created_at='2026-08-04T00:00:21Z';",
    ).unwrap();
    let preparing = productive_integration_rows(&connection, &units, &relationships).unwrap();
    assert_eq!(
        preparing["unit"].progress.as_ref().unwrap().phase,
        WorkUnitIntegrationProgressPhaseDto::Preparing
    );

    connection.execute_batch(
        "UPDATE accepted_work_unit_integrations SET stage='runtime_advanced',ref_advanced_at='2026-08-04T00:00:22Z',runtime_advanced_at='2026-08-04T00:00:23Z';",
    ).unwrap();
    let applying = productive_integration_rows(&connection, &units, &relationships).unwrap();
    assert_eq!(
        applying["unit"].progress.as_ref().unwrap().phase,
        WorkUnitIntegrationProgressPhaseDto::Applying
    );
    assert!(applying["unit"].success.is_none());
}

#[test]
fn productive_integration_projection_maps_safe_attention_without_terminal_facts() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    connection.execute_batch(
        "UPDATE accepted_work_unit_integrations SET stage='attention',attention_code='durable_replay_conflict',attention_recorded_at='2026-08-04T00:00:21Z';",
    ).unwrap();
    let projected = productive_integration_rows(&connection, &units, &relationships).unwrap();
    let attention = projected["unit"].attention.as_ref().unwrap();
    assert_eq!(attention.kind, WorkUnitIntegrationAttentionKindDto::Conflict);
    assert_eq!(attention.safe_code, WorkUnitIntegrationAttentionCodeDto::IntegrationConflict);
    assert!(projected["unit"].success.is_none());
    assert!(projected["unit"].settlement.is_none());

    connection.execute("UPDATE accepted_work_unit_integrations SET attention_code='C:\\private\\repository\\diagnostic'", []).unwrap();
    let sanitized = productive_integration_rows(&connection, &units, &relationships).unwrap();
    let json = serde_json::to_string(&sanitized["unit"]).unwrap();
    assert_eq!(sanitized["unit"].attention.as_ref().unwrap().kind, WorkUnitIntegrationAttentionKindDto::Failure);
    assert!(!json.contains("private"));
}

#[test]
fn productive_integration_projection_keeps_success_settlement_and_contribution_separate_and_private() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    seed_settled_productive_integration(&connection);
    let projected = productive_integration_rows(&connection, &units, &relationships).unwrap();
    let integration = &projected["unit"];
    assert_eq!(integration.success.as_ref().unwrap().recorded_at, "2026-08-04T00:00:25Z");
    assert_eq!(integration.settlement.as_ref().unwrap().settled_at, "2026-08-04T00:00:25Z");
    assert_eq!(integration.prerequisite_contribution.as_ref().unwrap().dependent_count, 1);
    let json = serde_json::to_string(integration).unwrap();
    for private in [
        "private-integration-id",
        "private-candidate-id",
        "private-intent-fingerprint",
        "private-integration-commit",
        "private-integration-tree",
        "private-ref",
    ] {
        assert!(!json.contains(private), "leaked {private}: {json}");
    }
}

#[test]
fn productive_integration_projection_rejects_order_unknown_stage_and_missing_terminal_bundle() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    connection.execute("UPDATE accepted_work_unit_integrations SET authorization_recorded_at='2026-08-04T00:00:19Z'", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("authorization"));

    connection.execute_batch("UPDATE accepted_work_unit_integrations SET authorization_recorded_at='2026-08-04T00:00:20Z',stage='unknown';").unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("unknown"));

    connection.execute_batch("UPDATE accepted_work_unit_integrations SET stage='settled',integration_commit_id='private-integration-commit',integration_tree_id='private-integration-tree',object_created_at='2026-08-04T00:00:21Z',ref_advanced_at='2026-08-04T00:00:22Z',runtime_advanced_at='2026-08-04T00:00:23Z',db_advanced_at='2026-08-04T00:00:24Z',settled_at='2026-08-04T00:00:25Z'; INSERT INTO work_unit_settlements VALUES('settlement','unit','private-integration-id','2026-08-04T00:00:25Z');").unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("lacks success evidence"));
}

#[test]
fn productive_integration_projection_rejects_contribution_before_settlement() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    seed_settled_productive_integration(&connection);
    connection.execute_batch(
        "UPDATE accepted_work_unit_integrations SET settled_at='2026-08-04T00:00:27Z';
         UPDATE work_unit_settlements SET settled_at='2026-08-04T00:00:27Z';
         UPDATE work_unit_prerequisite_contributions SET recorded_at='2026-08-04T00:00:26Z';",
    ).unwrap();

    assert!(productive_integration_rows(&connection, &units, &relationships)
        .unwrap_err()
        .contains("Prerequisite contribution precedes its prerequisite"));
}

#[test]
fn productive_integration_projection_rejects_orphan_duplicate_foreign_and_unaccepted_correlations() {
    let (connection, units, relationships) = productive_integration_projection_fixture();
    connection.execute("INSERT INTO accepted_work_unit_integration_evidence VALUES('orphan','missing','fingerprint','commit','tree','parent','candidate','private-ref','intent','2026-08-04T00:00:25Z')", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("orphaned"));

    connection.execute("DELETE FROM accepted_work_unit_integration_evidence", []).unwrap();
    seed_settled_productive_integration(&connection);
    connection.execute("INSERT INTO accepted_work_unit_integration_evidence VALUES('duplicate','private-integration-id','fingerprint-2','private-integration-commit','private-integration-tree','parent','private-candidate-id','private-ref','private-intent-fingerprint','2026-08-04T00:00:25Z')", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("duplicate success evidence"));

    connection.execute("DELETE FROM accepted_work_unit_integration_evidence WHERE evidence_id='duplicate'", []).unwrap();
    connection.execute("UPDATE accepted_work_unit_integration_evidence SET candidate_id='foreign-candidate'", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("foreign correlation"));

    connection.execute("UPDATE accepted_work_unit_integration_evidence SET candidate_id='private-candidate-id'", []).unwrap();
    connection.execute("UPDATE accepted_handler_candidates SET authority_id='foreign-authority'", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("accepted Handler authority"));

    connection.execute("UPDATE accepted_handler_candidates SET authority_id='private-authority'", []).unwrap();
    connection.execute("UPDATE work_unit_handler_decisions SET decision_variant='returned',implementation_accepted_at=NULL,implementation_returned_at='2026-08-04T00:00:19Z',retry_required_at='2026-08-04T00:00:19Z'", []).unwrap();
    assert!(productive_integration_rows(&connection, &units, &relationships).unwrap_err().contains("accepted Handler authority"));
}

fn productive_integration_projection_fixture() -> (
    rusqlite::Connection,
    Vec<WorkUnitDto>,
    Vec<WorkUnitRelationshipDto>,
) {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE accepted_work_unit_integrations (integration_id TEXT,work_unit_id TEXT,candidate_id TEXT,authority_id TEXT,target_ref_name TEXT,pre_object_id TEXT,pre_version INTEGER,candidate_commit_id TEXT,candidate_tree_id TEXT,baseline_object_id TEXT,intent_fingerprint TEXT,intent_recorded_at TEXT,authorization_recorded_at TEXT,commit_fingerprint TEXT,stage TEXT,integration_commit_id TEXT,integration_tree_id TEXT,object_created_at TEXT,ref_advanced_at TEXT,runtime_advanced_at TEXT,db_advanced_at TEXT,settled_at TEXT,notification_intent_recorded_at TEXT,notification_delivered_at TEXT,attention_code TEXT,attention_recorded_at TEXT);
         CREATE TABLE accepted_handler_candidates (candidate_id TEXT,work_unit_id TEXT,authority_id TEXT,pinned_at TEXT,attention_reason TEXT,review_invocation_id TEXT,decision_fingerprint TEXT);
         CREATE TABLE work_unit_handler_reviews (work_unit_id TEXT,review_invocation_id TEXT,semantic_judgment_variant TEXT,lifecycle_status TEXT,lifecycle_observed_at TEXT);
         CREATE TABLE work_unit_handler_decisions (work_unit_id TEXT,review_invocation_id TEXT,decision_variant TEXT,decision_fingerprint TEXT,decision_recorded_at TEXT,implementation_accepted_at TEXT,implementation_returned_at TEXT,retry_required_at TEXT,settlement_ready_at TEXT);
         CREATE TABLE accepted_work_unit_integration_evidence (evidence_id TEXT,integration_id TEXT,evidence_fingerprint TEXT,integration_commit_id TEXT,integration_tree_id TEXT,parent_object_id TEXT,candidate_id TEXT,target_ref_name TEXT,intent_fingerprint TEXT,recorded_at TEXT);
         CREATE TABLE work_unit_settlements (settlement_id TEXT,work_unit_id TEXT,integration_id TEXT,settled_at TEXT);
         CREATE TABLE work_unit_prerequisite_contributions (contribution_id TEXT,prerequisite_work_unit_id TEXT,dependent_work_unit_id TEXT,integration_id TEXT,relationship_id TEXT,recorded_at TEXT);
         CREATE TABLE work_unit_relationships (relationship_id TEXT,materialization_id TEXT,relationship_kind TEXT,from_id TEXT,to_id TEXT,ordinal INTEGER);
         INSERT INTO accepted_handler_candidates VALUES('private-candidate-id','unit','private-authority','2026-08-04T00:00:19Z',NULL,'review','decision');
         INSERT INTO work_unit_handler_reviews VALUES('unit','review','accept','completed','2026-08-04T00:00:18Z');
         INSERT INTO work_unit_handler_decisions VALUES('unit','review','accepted','decision','2026-08-04T00:00:19Z','2026-08-04T00:00:19Z',NULL,NULL,NULL);
         INSERT INTO accepted_work_unit_integrations VALUES('private-integration-id','unit','private-candidate-id','private-authority','private-ref','private-pre',1,'private-candidate-commit','private-candidate-tree','private-baseline','private-intent-fingerprint','2026-08-04T00:00:20Z','2026-08-04T00:00:20Z',NULL,'intent_reserved',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
         INSERT INTO work_unit_relationships VALUES('dependency','materialization','depends_on','dependent','unit',NULL);",
    ).unwrap();
    let mut unit = valid_work_unit_activation_projection();
    unit.work_unit_id = "unit".into();
    unit.materialization_id = "materialization".into();
    let mut dependent = valid_work_unit_activation_projection();
    dependent.work_unit_id = "dependent".into();
    dependent.materialization_id = "materialization".into();
    dependent.lane_ordinal = 1;
    let relationships = vec![WorkUnitRelationshipDto {
        relationship_id: "dependency".into(),
        materialization_id: "materialization".into(),
        relationship_kind: "depends_on".into(),
        from_id: "dependent".into(),
        to_id: "unit".into(),
        ordinal: None,
    }];
    (connection, vec![unit, dependent], relationships)
}

fn seed_settled_productive_integration(connection: &rusqlite::Connection) {
    connection.execute_batch(
        "UPDATE accepted_work_unit_integrations SET stage='settled',integration_commit_id='private-integration-commit',integration_tree_id='private-integration-tree',object_created_at='2026-08-04T00:00:21Z',ref_advanced_at='2026-08-04T00:00:22Z',runtime_advanced_at='2026-08-04T00:00:23Z',db_advanced_at='2026-08-04T00:00:24Z',settled_at='2026-08-04T00:00:25Z';
         INSERT INTO accepted_work_unit_integration_evidence VALUES('evidence','private-integration-id','private-evidence-fingerprint','private-integration-commit','private-integration-tree','private-parent','private-candidate-id','private-ref','private-intent-fingerprint','2026-08-04T00:00:25Z');
         INSERT INTO work_unit_settlements VALUES('settlement','unit','private-integration-id','2026-08-04T00:00:25Z');
         INSERT INTO work_unit_prerequisite_contributions VALUES('contribution','unit','dependent','private-integration-id','dependency','2026-08-04T00:00:25Z');",
    ).unwrap();
}
