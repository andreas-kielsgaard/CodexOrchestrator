use super::{
    build_storage::attempt_storage_key,
    domain::{
        BuildOutputId, BuildOutputStorageKey, ExecutableRelativePath, OperationAttemptId,
        OperationFailureCategory, OperationStage, RepositoryId, RetainedBuildOutput, ReviewBuildId,
        ReviewWorkspace, WorkspaceOwnership,
    },
};
use crate::{
    repository_context::RepositoryIdentity,
    worktree_application::{
        PhysicalWorktreeApplication, PhysicalWorktreeBuildRequest, PhysicalWorktreeBuildResult,
        PhysicalWorktreeDependencyPolicy, WorktreeApplicationError, WorktreeApplicationErrorKind,
    },
};
use std::path::{Component, Path, PathBuf};

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

/// Worktree Review adapter for dependency provisioning, physical compilation, and retained output.
/// Physical build/open mechanics belong to `worktree_application`; this adapter owns only Review
/// IDs, AppData layout, attempt stages, and the durable pointer to atomically published output.
pub(crate) struct ReviewBuildExecutor {
    application: PhysicalWorktreeApplication,
    review_root: PathBuf,
    repository_id: RepositoryId,
    dependency_cache_root: PathBuf,
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
        let dependency_cache_root = review_root.join("shared-cache").join("npm");
        Ok(Self {
            application: PhysicalWorktreeApplication,
            review_root,
            repository_id: RepositoryId::new(repository.id.as_str()).map_err(|error| {
                BuildExecutionFailure::new(
                    OperationStage::WorktreeProvisioning,
                    OperationFailureCategory::ProvisioningFailed,
                    error.to_string(),
                )
            })?,
            dependency_cache_root,
        })
    }

    pub(crate) fn execute(
        &self,
        build_id: &ReviewBuildId,
        attempt_id: &OperationAttemptId,
        workspace: &ReviewWorkspace,
    ) -> Result<RetainedBuildOutput, BuildExecutionFailure> {
        let attempt_key =
            attempt_storage_key(&self.repository_id, build_id, attempt_id).map_err(|error| {
                BuildExecutionFailure::new(
                    OperationStage::WorktreeProvisioning,
                    OperationFailureCategory::ProvisioningFailed,
                    error.to_string(),
                )
            })?;
        let attempt_root = self.review_root.join(attempt_key.as_str());
        let dependency_policy =
            dependency_policy(&workspace.ownership, &self.dependency_cache_root);
        let request = PhysicalWorktreeBuildRequest::new(
            PathBuf::from(workspace.location.as_str()),
            attempt_root,
            dependency_policy,
            "codex-orchestrator",
        )
        .map_err(application_failure)?;
        let result = self
            .application
            .build(&request)
            .map_err(application_failure)?;
        retained_output(
            &self.review_root,
            &request.attempt_root,
            build_id,
            attempt_id,
            &result,
        )
    }
}

fn dependency_policy(
    ownership: &WorkspaceOwnership,
    cache_root: &Path,
) -> PhysicalWorktreeDependencyPolicy {
    match ownership {
        WorkspaceOwnership::BorrowedExternal { .. } => {
            PhysicalWorktreeDependencyPolicy::UseExisting
        }
        WorkspaceOwnership::ManagedBranchWorktree { .. }
        | WorkspaceOwnership::OwnedBuildWorktree { .. } => {
            PhysicalWorktreeDependencyPolicy::Install {
                cache_root: cache_root.to_path_buf(),
            }
        }
    }
}

pub(crate) fn resolve_retained_output(
    review_root: &Path,
    workspace: &ReviewWorkspace,
    output: &RetainedBuildOutput,
) -> Result<PhysicalWorktreeBuildResult, String> {
    output.validate().map_err(|_| unavailable_output())?;
    let review_root = review_root
        .canonicalize()
        .map_err(|_| unavailable_output())?;
    let output_root = review_root
        .join(output.storage_key.as_str())
        .canonicalize()
        .map_err(|_| unavailable_output())?;
    if !output_root.is_dir() || !output_root.starts_with(&review_root) {
        return Err(unavailable_output());
    }
    let executable = output_root
        .join(output.executable_relative_path.as_str())
        .canonicalize()
        .map_err(|_| unavailable_output())?;
    if !executable.is_file() || !executable.starts_with(&output_root) {
        return Err(unavailable_output());
    }
    let attempt_root = output_root
        .parent()
        .ok_or_else(unavailable_output)?
        .to_path_buf();
    Ok(PhysicalWorktreeBuildResult {
        worktree_root: PathBuf::from(workspace.location.as_str()),
        log_path: attempt_root.join("build.log"),
        attempt_root,
        output_root,
        executable,
    })
}

fn retained_output(
    review_root: &Path,
    expected_attempt_root: &Path,
    build_id: &ReviewBuildId,
    attempt_id: &OperationAttemptId,
    result: &PhysicalWorktreeBuildResult,
) -> Result<RetainedBuildOutput, BuildExecutionFailure> {
    let review_root = review_root.canonicalize().map_err(|_| output_invalid())?;
    let attempt_root = result
        .attempt_root
        .canonicalize()
        .map_err(|_| output_invalid())?;
    let expected_attempt_root = expected_attempt_root
        .canonicalize()
        .map_err(|_| output_invalid())?;
    let output_root = result
        .output_root
        .canonicalize()
        .map_err(|_| output_missing())?;
    let executable = result
        .executable
        .canonicalize()
        .map_err(|_| output_missing())?;
    if attempt_root != expected_attempt_root
        || output_root.parent() != Some(attempt_root.as_path())
        || output_root.file_name().and_then(|name| name.to_str()) != Some("output")
        || !output_root.is_dir()
        || !executable.is_file()
        || !executable.starts_with(&output_root)
    {
        return Err(output_missing());
    }
    let storage_key = normalized_relative(&review_root, &output_root).ok_or_else(output_invalid)?;
    let executable_relative_path =
        normalized_relative(&output_root, &executable).ok_or_else(output_invalid)?;
    let output = RetainedBuildOutput {
        id: BuildOutputId::random(),
        build_id: build_id.clone(),
        attempt_id: attempt_id.clone(),
        storage_key: BuildOutputStorageKey::new(storage_key).map_err(|_| output_invalid())?,
        executable_relative_path: ExecutableRelativePath::new(executable_relative_path)
            .map_err(|_| output_invalid())?,
        published_at: chrono::Utc::now(),
    };
    output.validate().map_err(|_| output_invalid())?;
    Ok(output)
}

fn normalized_relative(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!components.is_empty()).then(|| components.join("/"))
}

fn application_failure(error: WorktreeApplicationError) -> BuildExecutionFailure {
    let category = match error.kind {
        WorktreeApplicationErrorKind::ToolchainUnavailable => {
            OperationFailureCategory::ToolchainUnavailable
        }
        WorktreeApplicationErrorKind::ExecutableUnavailable => {
            OperationFailureCategory::OutputMissing
        }
        WorktreeApplicationErrorKind::BuildFailed => OperationFailureCategory::CommandFailed,
        _ => OperationFailureCategory::ProvisioningFailed,
    };
    let stage = match error.kind {
        WorktreeApplicationErrorKind::ExecutableUnavailable => OperationStage::OutputPublication,
        WorktreeApplicationErrorKind::BuildFailed => OperationStage::Compilation,
        WorktreeApplicationErrorKind::ToolchainUnavailable => OperationStage::Compilation,
        _ => OperationStage::WorktreeProvisioning,
    };
    BuildExecutionFailure::new(stage, category, error.message)
}

fn output_missing() -> BuildExecutionFailure {
    BuildExecutionFailure::new(
        OperationStage::OutputPublication,
        OperationFailureCategory::OutputMissing,
        "Compilation completed without a retained application output.",
    )
}

fn output_invalid() -> BuildExecutionFailure {
    BuildExecutionFailure::new(
        OperationStage::OutputPublication,
        OperationFailureCategory::OutputPublicationFailed,
        "The retained application output is outside Worktree Review AppData.",
    )
}

fn unavailable_output() -> String {
    "The retained build output is unavailable or outside Worktree Review AppData.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::domain::{
        RepositoryId, WorkspaceId, WorkspaceLifecycle, WorkspaceOwnership, WorktreeAssociationId,
        WorktreeId, WorktreeLocation,
    };
    use std::fs;

    #[test]
    fn retained_output_requires_a_current_contained_executable() {
        let directory = tempfile::tempdir().unwrap();
        let review_root = directory.path().join("review");
        let attempt_root = review_root.join("repositories/repository/build-output/build/attempt");
        let output_root = attempt_root.join("output");
        let executable = output_root.join("cargo-target/debug/app.exe");
        let worktree_root = directory.path().join("worktree");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir_all(&worktree_root).unwrap();
        fs::write(&executable, b"app").unwrap();
        fs::write(attempt_root.join("build.log"), b"build").unwrap();
        let physical = PhysicalWorktreeBuildResult {
            worktree_root: worktree_root.clone(),
            attempt_root: attempt_root.clone(),
            output_root: output_root.clone(),
            log_path: attempt_root.join("build.log"),
            executable: executable.clone(),
        };
        let build_id = ReviewBuildId::new("build").unwrap();
        let attempt_id = OperationAttemptId::new("attempt").unwrap();
        let output = retained_output(
            &review_root,
            &attempt_root,
            &build_id,
            &attempt_id,
            &physical,
        )
        .unwrap();
        let workspace = ReviewWorkspace {
            id: WorkspaceId::new("workspace").unwrap(),
            repository_id: RepositoryId::new("repository").unwrap(),
            worktree_id: WorktreeId::new("worktree").unwrap(),
            location: WorktreeLocation::new(worktree_root.to_string_lossy()).unwrap(),
            ownership: WorkspaceOwnership::OwnedBuildWorktree { build_id },
            lifecycle: WorkspaceLifecycle::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(
            output.storage_key.as_str(),
            "repositories/repository/build-output/build/attempt/output"
        );
        assert!(resolve_retained_output(&review_root, &workspace, &output).is_ok());

        fs::remove_file(executable).unwrap();
        assert!(resolve_retained_output(&review_root, &workspace, &output).is_err());
    }

    #[test]
    fn borrowed_worktree_never_selects_dependency_installation() {
        let cache = PathBuf::from("C:/review-cache");
        let borrowed = WorkspaceOwnership::BorrowedExternal {
            association_id: WorktreeAssociationId::new("association").unwrap(),
        };
        let owned = WorkspaceOwnership::OwnedBuildWorktree {
            build_id: ReviewBuildId::new("build").unwrap(),
        };

        assert_eq!(
            dependency_policy(&borrowed, &cache),
            PhysicalWorktreeDependencyPolicy::UseExisting
        );
        assert_eq!(
            dependency_policy(&owned, &cache),
            PhysicalWorktreeDependencyPolicy::Install { cache_root: cache }
        );
    }
}
