//! Product-owned worktree test-instance identity, persistence, and process-tree ownership.
//!
//! This module does not schedule work, provision credentials, approve actions, or provide
//! pause/resume semantics. Projected configuration and observed runtime facts remain distinct.

mod application;
mod build_cache;
mod domain;
mod execution;
mod facade;
mod health;
mod ownership;
mod planning;
mod projection;
mod registry;

pub(crate) use application::WorktreeRuntimeApplication;
pub(crate) use domain::AuthoritySecret;
pub(crate) use execution::SystemActionExecutor;
#[allow(unused_imports)]
pub(crate) use facade::{
    HealthState, IsolatedTestRequest, RequestedTestInstance, RetainedTestSource, TestActionOutcome,
    TestActionProgress, TestActionProgressSink, TestActionResult, TestActionStage,
    TestInstanceError, TestInstanceErrorKind, TestInstanceHandle, TestInstancePhase,
    TestInstanceStatus, TestSourceRef, TestStartProgress, TestStartProgressSink, TestStartStage,
    VerifiedTestSource, WorktreeTestInstances,
};
pub(crate) use facade::{TestSourceResolver, WorktreeTestInstanceFacade};
pub(crate) use health::TcpHealthProbe;
#[cfg(not(windows))]
pub(crate) use ownership::UnsupportedProcessOwner;
#[cfg(windows)]
pub(crate) use ownership::WindowsJobProcessOwner;
pub(crate) use planning::{RuntimeSettings, SystemSourceInspector, ToolchainPrograms};
pub(crate) use registry::SqliteInstanceRegistry;

#[cfg(test)]
mod facade_tests;
#[cfg(test)]
mod tests;
