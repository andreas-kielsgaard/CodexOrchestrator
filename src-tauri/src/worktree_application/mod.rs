mod build;
mod domain;
mod open;

#[allow(unused_imports)]
pub(crate) use domain::{
    OpenOutcome, PhysicalWorktreeBuildRequest, PhysicalWorktreeBuildResult,
    WorktreeApplicationError, WorktreeApplicationErrorKind, WorktreeApplicationLaunchContext,
};

#[derive(Default)]
pub(crate) struct PhysicalWorktreeApplication;

impl PhysicalWorktreeApplication {
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
