mod build;
mod capture;
mod checkout;
mod domain;
mod git;
mod open;
pub(crate) mod transport;

use crate::repository_context::GitExecutable;
#[allow(unused_imports)]
pub(crate) use domain::{
    GitCommitId, OpenOutcome, PhysicalWorktreeAttachment, PhysicalWorktreeBuildRequest,
    PhysicalWorktreeBuildResult, PhysicalWorktreeCheckoutRequest, PhysicalWorktreeCheckoutResult,
    PhysicalWorktreeDependencyPolicy, VirtualCommitCaptureRequest, VirtualCommitCaptureResult,
    WorktreeApplicationError, WorktreeApplicationErrorKind, WorktreeApplicationLaunchContext,
};

#[derive(Default)]
pub(crate) struct PhysicalWorktreeApplication;

impl PhysicalWorktreeApplication {
    pub(crate) fn capture_virtual_commit(
        &self,
        git: &GitExecutable,
        request: &VirtualCommitCaptureRequest,
    ) -> Result<VirtualCommitCaptureResult, WorktreeApplicationError> {
        capture::capture_virtual_commit(git, request)
    }

    pub(crate) fn materialize_checkout(
        &self,
        git: &GitExecutable,
        request: &PhysicalWorktreeCheckoutRequest,
    ) -> Result<PhysicalWorktreeCheckoutResult, WorktreeApplicationError> {
        checkout::materialize_checkout(git, request)
    }

    pub(crate) fn build(
        &self,
        request: &PhysicalWorktreeBuildRequest,
    ) -> Result<PhysicalWorktreeBuildResult, WorktreeApplicationError> {
        build::build(request)
    }

    pub(crate) fn open(
        &self,
        build: &PhysicalWorktreeBuildResult,
        context: &WorktreeApplicationLaunchContext,
    ) -> Result<OpenOutcome, WorktreeApplicationError> {
        open::open(build, context)
    }
}
