use super::{
    artifact_store::ArtifactStore,
    domain::{
        OperationAttemptId, OperationFailureCategory, OperationStage, ReviewBuildId,
        ReviewWorkspace, VerifiedArtifactSet,
    },
};
use crate::{
    repository_context::RepositoryIdentity,
    worktree_application::{
        PhysicalWorktreeApplication, PhysicalWorktreeBuildRequest, WorktreeApplicationError,
        WorktreeApplicationErrorKind,
    },
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MAX_ARTIFACT_FILES: usize = 20_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuildExecutionFailure {
    pub(crate) stage: OperationStage,
    pub(crate) category: OperationFailureCategory,
    pub(crate) message: String,
}

impl BuildExecutionFailure {
    fn new(
        stage: OperationStage,
        category: OperationFailureCategory,
        message: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            category,
            message: message.into(),
        }
    }
}

/// Worktree Review adapter for dependency provisioning, physical compilation, and artifact copy.
/// Physical build/open mechanics belong to `worktree_application`; this adapter owns only Review
/// IDs, AppData layout, attempt stages, and artifact publication.
pub(crate) struct ReviewBuildExecutor {
    application: PhysicalWorktreeApplication,
    npm: PathBuf,
    build_output_root: PathBuf,
    shared_cache_root: PathBuf,
    dependency_log_root: PathBuf,
    artifact_store: ArtifactStore,
}

impl ReviewBuildExecutor {
    pub(crate) fn open(
        review_root: &Path,
        repository: &RepositoryIdentity,
    ) -> Result<Self, BuildExecutionFailure> {
        let review_root = review_root.canonicalize().map_err(|_| {
            BuildExecutionFailure::new(
                OperationStage::WorktreeProvisioning,
                OperationFailureCategory::ProvisioningFailed,
                "Worktree Review AppData storage is unavailable.",
            )
        })?;
        let repository_root = review_root
            .join("repositories")
            .join(repository.id.as_str());
        let build_output_root = repository_root.join("build-output");
        let dependency_log_root = repository_root.join("dependency-logs");
        let shared_cache_root = review_root.join("shared-cache");
        for directory in [&build_output_root, &dependency_log_root, &shared_cache_root] {
            fs::create_dir_all(directory).map_err(|_| {
                BuildExecutionFailure::new(
                    OperationStage::WorktreeProvisioning,
                    OperationFailureCategory::ProvisioningFailed,
                    "The Worktree Review build storage is unavailable.",
                )
            })?;
        }
        let artifact_store = ArtifactStore::open(review_root).map_err(|message| {
            BuildExecutionFailure::new(
                OperationStage::ArtifactPromotion,
                OperationFailureCategory::ProvisioningFailed,
                message,
            )
        })?;
        Ok(Self {
            application: PhysicalWorktreeApplication,
            npm: resolve_program("npm").ok_or_else(|| {
                BuildExecutionFailure::new(
                    OperationStage::DependencyProvisioning,
                    OperationFailureCategory::ToolchainUnavailable,
                    "npm is required to provision worktree dependencies.",
                )
            })?,
            build_output_root,
            shared_cache_root,
            dependency_log_root,
            artifact_store,
        })
    }

    pub(crate) fn execute(
        &self,
        build_id: &ReviewBuildId,
        attempt_id: &OperationAttemptId,
        workspace: &ReviewWorkspace,
    ) -> Result<VerifiedArtifactSet, BuildExecutionFailure> {
        let output_root = self
            .build_output_root
            .join(build_id.as_str())
            .join(attempt_id.as_str());
        let request = PhysicalWorktreeBuildRequest::new(
            PathBuf::from(workspace.location.as_str()),
            output_root,
            "codex-orchestrator",
        )
        .map_err(application_failure)?;
        let result = self
            .application
            .build(&request)
            .map_err(application_failure)?;
        let declared = declared_artifacts(&result.output_root)?;
        self.artifact_store
            .promote(
                build_id.clone(),
                attempt_id.clone(),
                &result.output_root,
                &declared,
            )
            .map_err(|message| {
                BuildExecutionFailure::new(
                    OperationStage::ArtifactPromotion,
                    OperationFailureCategory::ArtifactInvalid,
                    message,
                )
            })
    }

    pub(crate) fn provision_dependencies(
        &self,
        workspace: &ReviewWorkspace,
    ) -> Result<(), BuildExecutionFailure> {
        let root = PathBuf::from(workspace.location.as_str());
        let required = [
            "node_modules/typescript/bin/tsc",
            "node_modules/vite/bin/vite.js",
            "node_modules/@tauri-apps/cli/tauri.js",
        ];
        if required
            .iter()
            .all(|relative| root.join(relative).is_file())
        {
            return Ok(());
        }
        if !root.join("package-lock.json").is_file() {
            return Err(BuildExecutionFailure::new(
                OperationStage::DependencyProvisioning,
                OperationFailureCategory::ProvisioningFailed,
                "The selected source has no package-lock.json for a deterministic npm install.",
            ));
        }
        let log_path = self
            .dependency_log_root
            .join(format!("{}.log", workspace.id.as_str()));
        let log = fs::File::create(log_path).map_err(|_| dependency_log_failure())?;
        let error_log = log.try_clone().map_err(|_| dependency_log_failure())?;
        let status = Command::new(&self.npm)
            .args([
                "ci",
                "--prefer-offline",
                "--no-audit",
                "--no-fund",
                "--cache",
            ])
            .arg(self.shared_cache_root.join("npm"))
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(error_log))
            .status()
            .map_err(|_| {
                BuildExecutionFailure::new(
                    OperationStage::DependencyProvisioning,
                    OperationFailureCategory::ToolchainUnavailable,
                    "npm could not be started for the selected worktree.",
                )
            })?;
        if !status.success()
            || !required
                .iter()
                .all(|relative| root.join(relative).is_file())
        {
            return Err(BuildExecutionFailure::new(
                OperationStage::DependencyProvisioning,
                OperationFailureCategory::ProvisioningFailed,
                "Deterministic dependency provisioning did not complete successfully.",
            ));
        }
        Ok(())
    }
}

fn declared_artifacts(output_root: &Path) -> Result<Vec<PathBuf>, BuildExecutionFailure> {
    let mut declared = Vec::new();
    collect_regular_files(
        output_root,
        &output_root.join("frontend-dist"),
        &mut declared,
    )?;
    #[cfg(windows)]
    let executable = PathBuf::from("cargo-target/debug/codex-orchestrator.exe");
    #[cfg(not(windows))]
    let executable = PathBuf::from("cargo-target/debug/codex-orchestrator");
    if !output_root.join(&executable).is_file() {
        return Err(BuildExecutionFailure::new(
            OperationStage::ArtifactVerification,
            OperationFailureCategory::ArtifactMissing,
            "Compilation reported success but the application executable is missing.",
        ));
    }
    declared.push(executable);
    Ok(declared)
}

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), BuildExecutionFailure> {
    let entries = fs::read_dir(directory).map_err(|_| {
        BuildExecutionFailure::new(
            OperationStage::ArtifactVerification,
            OperationFailureCategory::ArtifactMissing,
            "Compilation reported success but the frontend output is missing.",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|_| artifact_invalid())?;
        let metadata = entry.file_type().map_err(|_| artifact_invalid())?;
        if metadata.is_symlink() {
            return Err(artifact_invalid());
        }
        if metadata.is_dir() {
            collect_regular_files(root, &entry.path(), output)?;
        } else if metadata.is_file() {
            output.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| artifact_invalid())?
                    .to_path_buf(),
            );
            if output.len() > MAX_ARTIFACT_FILES {
                return Err(BuildExecutionFailure::new(
                    OperationStage::ArtifactVerification,
                    OperationFailureCategory::ArtifactInvalid,
                    "The build produced too many distributable files to retain safely.",
                ));
            }
        } else {
            return Err(artifact_invalid());
        }
    }
    Ok(())
}

fn application_failure(error: WorktreeApplicationError) -> BuildExecutionFailure {
    let category = match error.kind {
        WorktreeApplicationErrorKind::ToolchainUnavailable => {
            OperationFailureCategory::ToolchainUnavailable
        }
        WorktreeApplicationErrorKind::ExecutableUnavailable => {
            OperationFailureCategory::ArtifactMissing
        }
        WorktreeApplicationErrorKind::BuildFailed => OperationFailureCategory::CommandFailed,
        _ => OperationFailureCategory::ProvisioningFailed,
    };
    let stage = match error.kind {
        WorktreeApplicationErrorKind::ExecutableUnavailable => OperationStage::ArtifactVerification,
        WorktreeApplicationErrorKind::BuildFailed => OperationStage::Compilation,
        WorktreeApplicationErrorKind::ToolchainUnavailable => OperationStage::Compilation,
        _ => OperationStage::WorktreeProvisioning,
    };
    BuildExecutionFailure::new(stage, category, error.message)
}

fn resolve_program(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    #[cfg(windows)]
    let names = [
        format!("{name}.cmd"),
        format!("{name}.exe"),
        name.to_owned(),
    ];
    #[cfg(not(windows))]
    let names = [name.to_owned()];
    env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

fn dependency_log_failure() -> BuildExecutionFailure {
    BuildExecutionFailure::new(
        OperationStage::DependencyProvisioning,
        OperationFailureCategory::ProvisioningFailed,
        "Dependency provisioning log storage is unavailable.",
    )
}

fn artifact_invalid() -> BuildExecutionFailure {
    BuildExecutionFailure::new(
        OperationStage::ArtifactVerification,
        OperationFailureCategory::ArtifactInvalid,
        "A build output entry is not a bounded regular file.",
    )
}
