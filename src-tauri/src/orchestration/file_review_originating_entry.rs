use super::{
    file_review_git_producer::{
        produce_file_review_from_git, FileReviewGitProducerError, ProduceFileReviewFromGit,
        ProducedFileReview,
    },
    initiated_sprint_git_authority::{
        BindInitiatedSprintGitAuthorityError, InitiatedSprintGitAuthorityService,
        WorktreeRuntimeGitComparison,
    },
    repository::{
        FileReviewGitCaptureAuthorizationError, FileReviewGitCaptureAuthorizationWrite,
        InitiatedSprintGitAuthorityError, SqliteOrchestrationRepository,
    },
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Internal application request. The durable Batch 15 authority is the sole caller input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProduceFileReviewForInitiatedSprint {
    pub(crate) authority_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FileReviewOriginatingEntryError {
    InvalidRequest,
    Unauthorized,
    RuntimeSourceUnavailable,
    RuntimeSourceStale,
    RuntimeSourceDirty,
    RuntimeSourceIncompatible,
    RuntimeEvidenceMismatch,
    ComparisonUnavailable,
    RepositoryUnavailable,
    RepositoryMismatch,
    GitObjectUnavailable,
    InvalidGitState,
    LimitsExceeded,
    IncompleteArtifact,
    Conflict,
    Unavailable,
}

pub(crate) struct FileReviewOriginatingEntryService {
    repository: Arc<SqliteOrchestrationRepository>,
    authority: InitiatedSprintGitAuthorityService,
}

impl FileReviewOriginatingEntryService {
    pub(crate) fn new(
        repository: Arc<SqliteOrchestrationRepository>,
        runtime: Arc<dyn WorktreeRuntimeGitComparison>,
    ) -> Self {
        Self {
            authority: InitiatedSprintGitAuthorityService::new(repository.clone(), runtime),
            repository,
        }
    }

    pub(crate) fn produce(
        &self,
        request: ProduceFileReviewForInitiatedSprint,
    ) -> Result<ProducedFileReview, FileReviewOriginatingEntryError> {
        let authority = self
            .authority
            .reauthorize(&request.authority_ref)
            .map_err(map_authority_error)?;
        let identity = stable_id("file-review-originating-entry", &authority.authority_id);
        let capture_authorization_id = stable_id("file-review-git-authorization", &identity);
        self.repository
            .store_file_review_git_capture_authorization(FileReviewGitCaptureAuthorizationWrite {
                capture_authorization_id: capture_authorization_id.clone(),
                idempotency_key: stable_id("file-review-git-authorization-request", &identity),
                epic_id: authority.epic_id,
                sprint_id: authority.sprint_id,
                provenance_id: authority.provenance_id,
                repository_id: authority.repository_id,
                repository_root: authority.repository_root,
                worktree_id: authority.worktree_id,
                worktree_root: authority.worktree_root,
                baseline_object_id: authority.baseline_object_id,
                current_object_id: authority.current_object_id,
            })
            .map_err(map_capture_authorization_error)?;
        produce_file_review_from_git(
            &self.repository,
            ProduceFileReviewFromGit {
                capture_authorization_id,
            },
        )
        .map_err(map_producer_error)
    }

    /// Product-context entry. No private authority, runtime identity, path, ref, or Git object is
    /// accepted from presentation.
    pub(crate) fn produce_for_sprint_context(
        &self,
        sprint_id: &str,
    ) -> Result<ProducedFileReview, FileReviewOriginatingEntryError> {
        let authority = self
            .repository
            .load_initiated_sprint_git_authority_for_sprint(sprint_id)
            .map_err(map_context_authority_error)?
            .ok_or(FileReviewOriginatingEntryError::Unauthorized)?;
        self.produce(ProduceFileReviewForInitiatedSprint {
            authority_ref: authority.authority_id,
        })
    }
}

fn map_context_authority_error(
    error: InitiatedSprintGitAuthorityError,
) -> FileReviewOriginatingEntryError {
    match error {
        InitiatedSprintGitAuthorityError::Invalid => {
            FileReviewOriginatingEntryError::InvalidRequest
        }
        InitiatedSprintGitAuthorityError::Forbidden => {
            FileReviewOriginatingEntryError::Unauthorized
        }
        InitiatedSprintGitAuthorityError::Conflict => FileReviewOriginatingEntryError::Conflict,
        InitiatedSprintGitAuthorityError::Unavailable => {
            FileReviewOriginatingEntryError::Unavailable
        }
    }
}

fn stable_id(kind: &str, seed: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"file-review-originating-entry/v1");
    hash.update((kind.len() as u64).to_be_bytes());
    hash.update(kind.as_bytes());
    hash.update((seed.len() as u64).to_be_bytes());
    hash.update(seed.as_bytes());
    format!("{kind}-{}", &format!("{:x}", hash.finalize())[..24])
}

fn map_authority_error(
    error: BindInitiatedSprintGitAuthorityError,
) -> FileReviewOriginatingEntryError {
    match error {
        BindInitiatedSprintGitAuthorityError::InvalidRequest => {
            FileReviewOriginatingEntryError::InvalidRequest
        }
        BindInitiatedSprintGitAuthorityError::SprintUnauthorized => {
            FileReviewOriginatingEntryError::Unauthorized
        }
        BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable => {
            FileReviewOriginatingEntryError::RuntimeSourceUnavailable
        }
        BindInitiatedSprintGitAuthorityError::RuntimeSourceStale => {
            FileReviewOriginatingEntryError::RuntimeSourceStale
        }
        BindInitiatedSprintGitAuthorityError::RuntimeSourceDirty => {
            FileReviewOriginatingEntryError::RuntimeSourceDirty
        }
        BindInitiatedSprintGitAuthorityError::RuntimeSourceIncompatible => {
            FileReviewOriginatingEntryError::RuntimeSourceIncompatible
        }
        BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch => {
            FileReviewOriginatingEntryError::RuntimeEvidenceMismatch
        }
        BindInitiatedSprintGitAuthorityError::ComparisonUnavailable => {
            FileReviewOriginatingEntryError::ComparisonUnavailable
        }
        BindInitiatedSprintGitAuthorityError::Conflict => FileReviewOriginatingEntryError::Conflict,
        BindInitiatedSprintGitAuthorityError::Unavailable => {
            FileReviewOriginatingEntryError::Unavailable
        }
    }
}

fn map_capture_authorization_error(
    error: FileReviewGitCaptureAuthorizationError,
) -> FileReviewOriginatingEntryError {
    match error {
        FileReviewGitCaptureAuthorizationError::Invalid => {
            FileReviewOriginatingEntryError::RuntimeEvidenceMismatch
        }
        FileReviewGitCaptureAuthorizationError::Forbidden => {
            FileReviewOriginatingEntryError::Unauthorized
        }
        FileReviewGitCaptureAuthorizationError::Conflict => {
            FileReviewOriginatingEntryError::Conflict
        }
        FileReviewGitCaptureAuthorizationError::Unavailable => {
            FileReviewOriginatingEntryError::Unavailable
        }
    }
}

fn map_producer_error(error: FileReviewGitProducerError) -> FileReviewOriginatingEntryError {
    match error {
        FileReviewGitProducerError::InvalidRequest => {
            FileReviewOriginatingEntryError::InvalidRequest
        }
        FileReviewGitProducerError::Unauthorized
        | FileReviewGitProducerError::InvalidAuthorization => {
            FileReviewOriginatingEntryError::Unauthorized
        }
        FileReviewGitProducerError::RepositoryUnavailable => {
            FileReviewOriginatingEntryError::RepositoryUnavailable
        }
        FileReviewGitProducerError::RepositoryMismatch => {
            FileReviewOriginatingEntryError::RepositoryMismatch
        }
        FileReviewGitProducerError::GitObjectUnavailable => {
            FileReviewOriginatingEntryError::GitObjectUnavailable
        }
        FileReviewGitProducerError::InvalidGitState => {
            FileReviewOriginatingEntryError::InvalidGitState
        }
        FileReviewGitProducerError::LimitsExceeded => {
            FileReviewOriginatingEntryError::LimitsExceeded
        }
        FileReviewGitProducerError::IncompleteArtifact => {
            FileReviewOriginatingEntryError::IncompleteArtifact
        }
        FileReviewGitProducerError::Conflict => FileReviewOriginatingEntryError::Conflict,
        FileReviewGitProducerError::Unavailable => FileReviewOriginatingEntryError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        orchestration::{
            application::OrchestrationApplication,
            initiated_sprint_git_authority::{
                BindInitiatedSprintGitAuthorityRequest, VerifiedRuntimeGitComparison,
            },
            repository::{
                FileReviewGitCaptureAuthorizationWrite, ScopedFileReviewLoad,
                StoreFileReviewGitCaptureAuthorizationResult, STORED_FILE_REVIEW_ARTIFACT_V1,
            },
        },
        storage::{configure_sqlite_connection, initialize_active_database},
    };
    use rusqlite::Connection;
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::Mutex,
    };

    struct RealComparisonPort {
        expected: Mutex<Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError>>,
    }

    impl WorktreeRuntimeGitComparison for RealComparisonPort {
        fn resolve_verified_comparison(
            &self,
            runtime_instance_ref: &str,
        ) -> Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError> {
            let expected = self.expected.lock().unwrap().clone()?;
            if expected.runtime_instance_ref != runtime_instance_ref {
                return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
            }
            let repository_root = PathBuf::from(&expected.repository_root)
                .canonicalize()
                .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
            let worktree_root = PathBuf::from(&expected.worktree_root)
                .canonicalize()
                .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
            if canonical_root(&repository_root) != expected.repository_root
                || canonical_root(&worktree_root) != expected.worktree_root
                || git_path(&repository_root, &["rev-parse", "--git-common-dir"])
                    != PathBuf::from(&expected.repository_common_dir)
                || git_path(&worktree_root, &["rev-parse", "--git-common-dir"])
                    != PathBuf::from(&expected.repository_common_dir)
                || git_text(
                    &worktree_root,
                    &[
                        "rev-parse",
                        "--verify",
                        &format!("{}^{{commit}}", expected.baseline_object_id),
                    ],
                ) != expected.baseline_object_id
                || git_text(&worktree_root, &["rev-parse", "--verify", "HEAD^{commit}"])
                    != expected.current_object_id
            {
                return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
            }
            Ok(expected)
        }
    }

    struct RealComparison {
        _temp: tempfile::TempDir,
        main_root: PathBuf,
        worktree_root: PathBuf,
        baseline: String,
        current: String,
        port: Arc<RealComparisonPort>,
    }

    fn real_comparison() -> RealComparison {
        let temp = tempfile::tempdir().unwrap();
        let main_root = temp.path().join("main");
        let worktree_root = temp.path().join("review-worktree");
        fs::create_dir(&main_root).unwrap();
        git(&main_root, &["init"]);
        git(
            &main_root,
            &["config", "user.name", "File Review Entry Test"],
        );
        git(
            &main_root,
            &["config", "user.email", "file-review-entry@example.invalid"],
        );
        git(&main_root, &["config", "commit.gpgsign", "false"]);
        write(&main_root, "modified.txt", b"before\nline\n");
        write(&main_root, "deleted.md", b"# Gone\n\nBody\n");
        write(&main_root, "docs/old-name.md", b"# Retained\n");
        write(&main_root, "binary.bin", &[0, 1, 2]);
        write(&main_root, "unsupported.dat", &[0xff, 0xfe]);
        git(&main_root, &["add", "-A"]);
        git(&main_root, &["commit", "-m", "baseline"]);
        let baseline = git_text(&main_root, &["rev-parse", "HEAD"]);
        git(
            &main_root,
            &[
                "worktree",
                "add",
                "-b",
                "review-candidate",
                worktree_root.to_str().unwrap(),
                &baseline,
            ],
        );
        write(&worktree_root, "modified.txt", b"after\nline\n");
        fs::remove_file(worktree_root.join("deleted.md")).unwrap();
        git(
            &worktree_root,
            &["mv", "docs/old-name.md", "docs/new-name.md"],
        );
        write(&worktree_root, "added.md", b"# Added\n\nReview text.\n");
        write(&worktree_root, "binary.bin", &[0, 9, 2]);
        write(&worktree_root, "unsupported.dat", &[0xff, 0xfd]);
        git(&worktree_root, &["add", "-A"]);
        git(&worktree_root, &["commit", "-m", "current"]);
        let current = git_text(&worktree_root, &["rev-parse", "HEAD"]);
        let common = git_path(&main_root, &["rev-parse", "--git-common-dir"]);
        let expected = VerifiedRuntimeGitComparison {
            repository_id: "repository-real".into(),
            repository_root: canonical_root(&main_root),
            repository_common_dir: canonical_root(&common),
            worktree_id: "worktree-real".into(),
            worktree_root: canonical_root(&worktree_root),
            baseline_object_id: baseline.clone(),
            current_object_id: current.clone(),
            runtime_instance_ref: "runtime-real".into(),
            runtime_source_ref: "source-real".into(),
            source_fingerprint: "1".repeat(64),
        };
        RealComparison {
            _temp: temp,
            main_root,
            worktree_root,
            baseline,
            current,
            port: Arc::new(RealComparisonPort {
                expected: Mutex::new(Ok(expected)),
            }),
        }
    }

    #[test]
    fn produces_complete_review_from_private_authority_and_replays_after_main_advances() {
        let comparison = real_comparison();
        let (repository, authority_ref, _database_path) = bound_repository(&comparison);

        write(&comparison.main_root, "main-advanced.txt", b"later\n");
        git(&comparison.main_root, &["add", "-A"]);
        git(&comparison.main_root, &["commit", "-m", "advance main"]);
        assert_ne!(
            git_text(&comparison.main_root, &["rev-parse", "HEAD"]),
            comparison.baseline
        );

        let service =
            FileReviewOriginatingEntryService::new(repository.clone(), comparison.port.clone());
        let first = service
            .produce(ProduceFileReviewForInitiatedSprint {
                authority_ref: authority_ref.clone(),
            })
            .unwrap();
        assert!(!first.idempotent_replay);
        assert_eq!(first.changed_file_count, 6);

        let application = OrchestrationApplication::new(repository.clone());
        let document = match application
            .load_scoped_file_review(&first.opaque_reference)
            .unwrap()
        {
            ScopedFileReviewLoad::Available { document } => document,
            other => panic!("unexpected File Review load: {other:?}"),
        };
        assert_eq!(document.document_ref_id, first.document_ref_id);
        assert_eq!(document.artifact_id, first.artifact_id);
        assert_eq!(
            document
                .changed_files
                .iter()
                .map(|file| (file.display_name.as_str(), file.change_kind.as_str()))
                .collect::<Vec<_>>(),
            [
                ("added.md", "added"),
                ("binary.bin", "modified"),
                ("deleted.md", "deleted"),
                ("docs/new-name.md", "renamed"),
                ("modified.txt", "modified"),
                ("unsupported.dat", "modified"),
            ]
        );
        assert_eq!(
            document.changed_files[3].previous_display_name.as_deref(),
            Some("docs/old-name.md")
        );
        let payload: serde_json::Value = serde_json::from_slice(&document.payload).unwrap();
        assert_eq!(payload["contractVersion"], STORED_FILE_REVIEW_ARTIFACT_V1);
        let files = payload["files"].as_array().unwrap();
        assert_eq!(files[1]["content"]["encoding"], "binary");
        assert_eq!(files[5]["content"]["encoding"], "unsupported");

        let replay = service
            .produce(ProduceFileReviewForInitiatedSprint { authority_ref })
            .unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(
            replay,
            ProducedFileReview {
                idempotent_replay: true,
                ..first
            }
        );

        let query_json = serde_json::to_string(&application.native_query().unwrap()).unwrap();
        assert!(query_json.contains(&replay.document_ref_id));
        assert!(!query_json.contains("runtime-real"));
        assert!(!query_json.contains(comparison.main_root.to_string_lossy().as_ref()));
        assert!(!query_json.contains(comparison.worktree_root.to_string_lossy().as_ref()));
    }

    #[test]
    fn product_context_resolves_private_authority_by_initiated_sprint() {
        let comparison = real_comparison();
        let (repository, _authority_ref, _database_path) = bound_repository(&comparison);
        let service = FileReviewOriginatingEntryService::new(repository, comparison.port.clone());

        let first = service.produce_for_sprint_context("sprint-1").unwrap();
        assert!(!first.idempotent_replay);
        let replay = service.produce_for_sprint_context("sprint-1").unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.opaque_reference, first.opaque_reference);
        assert_eq!(
            service.produce_for_sprint_context("sprint-2"),
            Err(FileReviewOriginatingEntryError::Unauthorized)
        );
        assert_eq!(
            service.produce_for_sprint_context(" "),
            Err(FileReviewOriginatingEntryError::InvalidRequest)
        );
    }

    #[test]
    fn rejects_stale_tampered_cross_epic_and_replaced_paths_without_file_review_facts() {
        for failure in ["stale", "tampered", "cross-epic", "path-replaced"] {
            let comparison = real_comparison();
            let (repository, authority_ref, database_path) = bound_repository(&comparison);
            match failure {
                "stale" => {
                    *comparison.port.expected.lock().unwrap() =
                        Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceStale);
                }
                "tampered" => {
                    Connection::open(&database_path).unwrap().execute(
                        "UPDATE initiated_sprint_git_authorities SET source_fingerprint='tampered' WHERE authority_id=?1",
                        [&authority_ref],
                    ).unwrap();
                }
                "cross-epic" => {
                    Connection::open(&database_path).unwrap().execute(
                        "UPDATE initiated_sprint_git_authorities SET epic_id='epic-2' WHERE authority_id=?1",
                        [&authority_ref],
                    ).unwrap();
                }
                "path-replaced" => {
                    git(
                        &comparison.main_root,
                        &[
                            "worktree",
                            "remove",
                            "--force",
                            comparison.worktree_root.to_str().unwrap(),
                        ],
                    );
                    fs::create_dir_all(&comparison.worktree_root).unwrap();
                    git(&comparison.worktree_root, &["init"]);
                }
                _ => unreachable!(),
            }
            let service =
                FileReviewOriginatingEntryService::new(repository.clone(), comparison.port.clone());
            assert!(service
                .produce(ProduceFileReviewForInitiatedSprint { authority_ref })
                .is_err());
            assert_eq!(
                file_review_fact_counts(&database_path),
                [0, 0, 0],
                "{failure}"
            );
        }
    }

    #[test]
    fn limit_and_authorization_conflict_fail_without_partial_file_review_facts() {
        let comparison = real_comparison();
        write(
            &comparison.worktree_root,
            "oversized.txt",
            &vec![b'x'; 256_001],
        );
        git(&comparison.worktree_root, &["add", "-A"]);
        git(&comparison.worktree_root, &["commit", "-m", "oversized"]);
        let oversized_current = git_text(&comparison.worktree_root, &["rev-parse", "HEAD"]);
        comparison
            .port
            .expected
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .current_object_id = oversized_current;
        let (repository, authority_ref, database_path) = bound_repository(&comparison);
        let service =
            FileReviewOriginatingEntryService::new(repository.clone(), comparison.port.clone());
        assert_eq!(
            service.produce(ProduceFileReviewForInitiatedSprint { authority_ref }),
            Err(FileReviewOriginatingEntryError::LimitsExceeded)
        );
        assert_eq!(file_review_fact_counts(&database_path), [0, 0, 0]);

        let comparison = real_comparison();
        let (repository, authority_ref, database_path) = bound_repository(&comparison);
        let identity = stable_id("file-review-originating-entry", &authority_ref);
        let capture_id = stable_id("file-review-git-authorization", &identity);
        let expected = comparison.port.expected.lock().unwrap().clone().unwrap();
        assert_eq!(
            repository
                .store_file_review_git_capture_authorization(
                    FileReviewGitCaptureAuthorizationWrite {
                        capture_authorization_id: capture_id,
                        idempotency_key: stable_id(
                            "file-review-git-authorization-request",
                            &identity,
                        ),
                        epic_id: "epic-1".into(),
                        sprint_id: "sprint-1".into(),
                        provenance_id: "provenance-1".into(),
                        repository_id: expected.repository_id,
                        repository_root: expected.repository_root,
                        worktree_id: expected.worktree_id,
                        worktree_root: expected.worktree_root,
                        baseline_object_id: expected.baseline_object_id,
                        current_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    },
                )
                .unwrap(),
            StoreFileReviewGitCaptureAuthorizationResult::Stored
        );
        let service =
            FileReviewOriginatingEntryService::new(repository.clone(), comparison.port.clone());
        assert_eq!(
            service.produce(ProduceFileReviewForInitiatedSprint { authority_ref }),
            Err(FileReviewOriginatingEntryError::Conflict)
        );
        assert_eq!(file_review_fact_counts(&database_path), [0, 0, 0]);
    }

    fn bound_repository(
        comparison: &RealComparison,
    ) -> (Arc<SqliteOrchestrationRepository>, String, PathBuf) {
        let path = comparison._temp.path().join("active-v3.sqlite");
        let connection = Connection::open(&path).unwrap();
        configure_sqlite_connection(&connection).unwrap();
        initialize_active_database(&connection).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES ('provenance-1','command-1','result-1','event-1','t'),('provenance-2','command-2','result-2','event-2','t'); INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('initiation-1','command-1','result-1','event-1','provenance-1','draft-1','revision-1','snapshot-1','epic-1','t'),('initiation-2','command-2','result-2','event-2','provenance-2','draft-2','revision-2','snapshot-2','epic-2','t'); INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES ('sprint-1','epic-1',0,'One','Move','[]','plan-1','plan-revision-1'),('sprint-2','epic-2',0,'Two','Move','[]','plan-2','plan-revision-2');").unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let repository = Arc::new(SqliteOrchestrationRepository::new(connection).unwrap());
        let authority =
            InitiatedSprintGitAuthorityService::new(repository.clone(), comparison.port.clone())
                .bind(BindInitiatedSprintGitAuthorityRequest {
                    sprint_id: "sprint-1".into(),
                    runtime_instance_ref: "runtime-real".into(),
                    idempotency_key: "entry-authority".into(),
                })
                .unwrap();
        (repository, authority.authority_ref, path)
    }

    fn file_review_fact_counts(database_path: &Path) -> [i64; 3] {
        let connection = Connection::open(database_path).unwrap();
        [
            "file_review_documents",
            "file_review_changed_files",
            "stored_file_review_artifacts",
        ]
        .map(|table| {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap()
        })
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn git_text(root: &Path, args: &[&str]) -> String {
        String::from_utf8(git(root, args))
            .unwrap()
            .trim()
            .to_owned()
    }

    fn git_path(root: &Path, args: &[&str]) -> PathBuf {
        let value = PathBuf::from(git_text(root, args));
        let value = if value.is_absolute() {
            value
        } else {
            root.join(value)
        };
        value.canonicalize().unwrap()
    }

    fn canonical_root(path: &Path) -> String {
        path.canonicalize().unwrap().to_string_lossy().into_owned()
    }
}
