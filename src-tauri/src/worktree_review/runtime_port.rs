use crate::worktree_runtime::{
    HealthState, IsolatedTestRequest, RetainedTestSource, TestActionOutcome, TestActionProgress,
    TestActionProgressSink, TestActionResult, TestActionStage, TestInstanceError,
    TestInstanceErrorKind, TestInstanceHandle, TestInstancePhase, TestInstanceStatus,
    TestSourceRef, TestStartProgress, TestStartProgressSink, TestStartStage, VerifiedTestSource,
    WorktreeTestInstances,
};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ReviewSourceRef(String);

impl ReviewSourceRef {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, ReviewRuntimeError> {
        let value = value.into();
        TestSourceRef::new(value.clone()).map_err(ReviewRuntimeError::from)?;
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ReviewInstanceHandle(String);

impl ReviewInstanceHandle {
    pub(crate) fn from_opaque(value: impl Into<String>) -> Result<Self, ReviewRuntimeError> {
        let value = value.into();
        TestInstanceHandle::from_opaque(value.clone()).map_err(ReviewRuntimeError::from)?;
        Ok(Self(value))
    }

    pub(crate) fn opaque_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewInstanceRequest {
    pub(crate) source: ReviewSourceRef,
    pub(crate) purpose: String,
}

impl ReviewInstanceRequest {
    pub(crate) fn new(
        source: ReviewSourceRef,
        purpose: impl Into<String>,
    ) -> Result<Self, ReviewRuntimeError> {
        let purpose = purpose.into();
        IsolatedTestRequest::new(
            TestSourceRef::new(source.0.clone()).map_err(ReviewRuntimeError::from)?,
            purpose.clone(),
        )
        .map_err(ReviewRuntimeError::from)?;
        Ok(Self { source, purpose })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewInstancePhase {
    Prepared,
    Starting,
    Running,
    Stopping,
    Stopped,
    Recovering,
    Recovered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewHealth {
    NotObserved,
    Healthy,
    Unhealthy,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewInstanceStatus {
    pub(crate) phase: ReviewInstancePhase,
    pub(crate) health: ReviewHealth,
    pub(crate) stale: bool,
    pub(crate) source_current: bool,
    pub(crate) build_reusable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RequestedReviewInstance {
    pub(crate) handle: ReviewInstanceHandle,
    pub(crate) status: ReviewInstanceStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewBuildOutcome {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewBuildResult {
    pub(crate) outcome: ReviewBuildOutcome,
    pub(crate) status: ReviewInstanceStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RetainedReviewSource {
    pub(crate) current_object_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedReviewSource {
    pub(crate) worktree_path: PathBuf,
    pub(crate) current_object_id: String,
    pub(crate) source_fingerprint: String,
    pub(crate) clean: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewBuildStage {
    SourceInspection,
    Typecheck,
    FrontendBuild,
    TauriCompileLink,
    BuildReuse,
    Finalizing,
}

pub(crate) struct ReviewBuildProgress<'a> {
    pub(crate) stage: ReviewBuildStage,
    pub(crate) output: Option<&'a str>,
}

pub(crate) trait ReviewBuildProgressSink: Send + Sync {
    fn progress(&self, progress: ReviewBuildProgress<'_>);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewStartStage {
    Reservation,
    SupportingServices,
    NativeStart,
    WaitingForWindow,
    Ready,
}

pub(crate) struct ReviewStartProgress<'a> {
    pub(crate) stage: ReviewStartStage,
    pub(crate) output: Option<&'a str>,
}

pub(crate) trait ReviewStartProgressSink: Send + Sync {
    fn progress(&self, progress: ReviewStartProgress<'_>);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewRuntimeErrorKind {
    InvalidRequest,
    NotFound,
    Unauthorized,
    InvalidState,
    OperationInProgress,
    Conflict,
    BuildRequired,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewRuntimeError {
    pub(crate) kind: ReviewRuntimeErrorKind,
    pub(crate) message: String,
}

impl From<TestInstanceError> for ReviewRuntimeError {
    fn from(error: TestInstanceError) -> Self {
        Self {
            kind: match error.kind {
                TestInstanceErrorKind::InvalidRequest => ReviewRuntimeErrorKind::InvalidRequest,
                TestInstanceErrorKind::NotFound => ReviewRuntimeErrorKind::NotFound,
                TestInstanceErrorKind::Unauthorized => ReviewRuntimeErrorKind::Unauthorized,
                TestInstanceErrorKind::InvalidState => ReviewRuntimeErrorKind::InvalidState,
                TestInstanceErrorKind::OperationInProgress => {
                    ReviewRuntimeErrorKind::OperationInProgress
                }
                TestInstanceErrorKind::Conflict => ReviewRuntimeErrorKind::Conflict,
                TestInstanceErrorKind::BuildRequired => ReviewRuntimeErrorKind::BuildRequired,
                TestInstanceErrorKind::Unavailable => ReviewRuntimeErrorKind::Unavailable,
            },
            message: error.message,
        }
    }
}

pub(crate) trait ReviewInstanceRuntime: Send + Sync {
    fn request(
        &self,
        request: ReviewInstanceRequest,
    ) -> Result<RequestedReviewInstance, ReviewRuntimeError>;
    fn build_with_progress(
        &self,
        handle: &ReviewInstanceHandle,
        progress: &dyn ReviewBuildProgressSink,
    ) -> Result<ReviewBuildResult, ReviewRuntimeError>;
    fn start_with_progress(
        &self,
        handle: &ReviewInstanceHandle,
        progress: &dyn ReviewStartProgressSink,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>;
    fn status(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>;
    fn focus(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>;
    fn stop(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>;
    fn recover(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>;
    fn retained_source(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<RetainedReviewSource, ReviewRuntimeError>;
    fn verified_source(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<VerifiedReviewSource, ReviewRuntimeError>;
    fn cleanup(&self, handle: &ReviewInstanceHandle) -> Result<(), ReviewRuntimeError>;
}

pub(crate) trait IntoReviewInstanceRuntime {
    fn into_review_instance_runtime(self) -> Arc<dyn ReviewInstanceRuntime>;
}

impl<T> IntoReviewInstanceRuntime for Arc<T>
where
    T: WorktreeTestInstances + ?Sized + 'static,
{
    fn into_review_instance_runtime(self) -> Arc<dyn ReviewInstanceRuntime> {
        Arc::new(WorktreeRuntimeAdapter { inner: self })
    }
}

struct WorktreeRuntimeAdapter<T: WorktreeTestInstances + ?Sized> {
    inner: Arc<T>,
}

impl<T> WorktreeRuntimeAdapter<T>
where
    T: WorktreeTestInstances + ?Sized,
{
    fn handle(handle: &ReviewInstanceHandle) -> Result<TestInstanceHandle, ReviewRuntimeError> {
        TestInstanceHandle::from_opaque(handle.0.clone()).map_err(ReviewRuntimeError::from)
    }
}

impl<T> ReviewInstanceRuntime for WorktreeRuntimeAdapter<T>
where
    T: WorktreeTestInstances + ?Sized,
{
    fn request(
        &self,
        request: ReviewInstanceRequest,
    ) -> Result<RequestedReviewInstance, ReviewRuntimeError> {
        let requested = self
            .inner
            .request(
                IsolatedTestRequest::new(
                    TestSourceRef::new(request.source.0).map_err(ReviewRuntimeError::from)?,
                    request.purpose,
                )
                .map_err(ReviewRuntimeError::from)?,
            )
            .map_err(ReviewRuntimeError::from)?;
        Ok(RequestedReviewInstance {
            handle: ReviewInstanceHandle(requested.handle.opaque_ref().to_owned()),
            status: requested.status.into(),
        })
    }

    fn build_with_progress(
        &self,
        handle: &ReviewInstanceHandle,
        progress: &dyn ReviewBuildProgressSink,
    ) -> Result<ReviewBuildResult, ReviewRuntimeError> {
        let adapter = BuildProgressAdapter { target: progress };
        self.inner
            .build_with_progress(&Self::handle(handle)?, &adapter)
            .map(ReviewBuildResult::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn start_with_progress(
        &self,
        handle: &ReviewInstanceHandle,
        progress: &dyn ReviewStartProgressSink,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError> {
        let adapter = StartProgressAdapter { target: progress };
        self.inner
            .start_with_progress(&Self::handle(handle)?, &adapter)
            .map(ReviewInstanceStatus::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn status(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError> {
        self.inner
            .status(&Self::handle(handle)?)
            .map(ReviewInstanceStatus::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn focus(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError> {
        self.inner
            .focus(&Self::handle(handle)?)
            .map(ReviewInstanceStatus::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn stop(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError> {
        self.inner
            .stop(&Self::handle(handle)?)
            .map(ReviewInstanceStatus::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn recover(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<ReviewInstanceStatus, ReviewRuntimeError> {
        self.inner
            .recover(&Self::handle(handle)?)
            .map(ReviewInstanceStatus::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn retained_source(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<RetainedReviewSource, ReviewRuntimeError> {
        self.inner
            .retained_source(&Self::handle(handle)?)
            .map(RetainedReviewSource::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn verified_source(
        &self,
        handle: &ReviewInstanceHandle,
    ) -> Result<VerifiedReviewSource, ReviewRuntimeError> {
        self.inner
            .verified_source(&Self::handle(handle)?)
            .map(VerifiedReviewSource::from)
            .map_err(ReviewRuntimeError::from)
    }

    fn cleanup(&self, handle: &ReviewInstanceHandle) -> Result<(), ReviewRuntimeError> {
        self.inner
            .cleanup(&Self::handle(handle)?)
            .map_err(ReviewRuntimeError::from)
    }
}

impl From<TestInstanceStatus> for ReviewInstanceStatus {
    fn from(status: TestInstanceStatus) -> Self {
        Self {
            phase: match status.phase {
                TestInstancePhase::Prepared => ReviewInstancePhase::Prepared,
                TestInstancePhase::Starting => ReviewInstancePhase::Starting,
                TestInstancePhase::Running => ReviewInstancePhase::Running,
                TestInstancePhase::Stopping => ReviewInstancePhase::Stopping,
                TestInstancePhase::Stopped => ReviewInstancePhase::Stopped,
                TestInstancePhase::Recovering => ReviewInstancePhase::Recovering,
                TestInstancePhase::Recovered => ReviewInstancePhase::Recovered,
            },
            health: match status.health {
                HealthState::NotObserved => ReviewHealth::NotObserved,
                HealthState::Healthy => ReviewHealth::Healthy,
                HealthState::Unhealthy => ReviewHealth::Unhealthy,
                HealthState::Closed => ReviewHealth::Closed,
            },
            stale: status.stale,
            source_current: status.source_current,
            build_reusable: status.build_reusable,
        }
    }
}

impl From<TestActionResult> for ReviewBuildResult {
    fn from(result: TestActionResult) -> Self {
        Self {
            outcome: match result.outcome {
                TestActionOutcome::Passed => ReviewBuildOutcome::Passed,
                TestActionOutcome::Failed => ReviewBuildOutcome::Failed,
            },
            status: result.status.into(),
        }
    }
}

impl From<RetainedTestSource> for RetainedReviewSource {
    fn from(source: RetainedTestSource) -> Self {
        Self {
            current_object_id: source.current_object_id,
        }
    }
}

impl From<VerifiedTestSource> for VerifiedReviewSource {
    fn from(source: VerifiedTestSource) -> Self {
        Self {
            worktree_path: source.worktree_path,
            current_object_id: source.current_object_id,
            source_fingerprint: source.source_fingerprint,
            clean: source.clean,
        }
    }
}

struct BuildProgressAdapter<'a> {
    target: &'a dyn ReviewBuildProgressSink,
}

impl TestActionProgressSink for BuildProgressAdapter<'_> {
    fn progress(&self, progress: TestActionProgress<'_>) {
        self.target.progress(ReviewBuildProgress {
            stage: match progress.stage {
                TestActionStage::SourceInspection => ReviewBuildStage::SourceInspection,
                TestActionStage::Typecheck => ReviewBuildStage::Typecheck,
                TestActionStage::FrontendBuild => ReviewBuildStage::FrontendBuild,
                TestActionStage::TauriCompileLink => ReviewBuildStage::TauriCompileLink,
                TestActionStage::BuildReuse => ReviewBuildStage::BuildReuse,
                TestActionStage::Finalizing => ReviewBuildStage::Finalizing,
            },
            output: progress.output,
        });
    }
}

struct StartProgressAdapter<'a> {
    target: &'a dyn ReviewStartProgressSink,
}

impl TestStartProgressSink for StartProgressAdapter<'_> {
    fn progress(&self, progress: TestStartProgress<'_>) {
        self.target.progress(ReviewStartProgress {
            stage: match progress.stage {
                TestStartStage::Reservation => ReviewStartStage::Reservation,
                TestStartStage::SupportingServices => ReviewStartStage::SupportingServices,
                TestStartStage::NativeStart => ReviewStartStage::NativeStart,
                TestStartStage::WaitingForWindow => ReviewStartStage::WaitingForWindow,
                TestStartStage::Ready => ReviewStartStage::Ready,
            },
            output: progress.output,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_runtime_status_without_leaking_runtime_vocabulary() {
        let status = ReviewInstanceStatus::from(TestInstanceStatus {
            phase: TestInstancePhase::Running,
            health: HealthState::Healthy,
            stale: false,
            source_current: true,
            build_reusable: true,
        });

        assert_eq!(status.phase, ReviewInstancePhase::Running);
        assert_eq!(status.health, ReviewHealth::Healthy);
        assert!(status.build_reusable);
    }
}
