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

#[cfg(any(debug_assertions, test))]
pub(crate) use application::WorktreeRuntimeApplication;
#[cfg(any(debug_assertions, test))]
pub(crate) use domain::AuthoritySecret;
#[cfg(any(debug_assertions, test))]
pub(crate) use execution::SystemActionExecutor;
#[allow(unused_imports)]
pub(crate) use facade::{
    HealthState, IsolatedTestRequest, RequestedTestInstance, TestActionOutcome, TestActionProgress,
    TestActionProgressSink, TestActionResult, TestActionStage, TestInstanceError,
    TestInstanceErrorKind, TestInstanceHandle, TestInstancePhase, TestInstanceStatus,
    TestSourceRef, TestStartProgress, TestStartProgressSink, TestStartStage, VerifiedTestSource,
    WorktreeTestInstances,
};
#[cfg(any(debug_assertions, test))]
pub(crate) use facade::{TestSourceResolver, WorktreeTestInstanceFacade};
#[cfg(any(debug_assertions, test))]
pub(crate) use health::TcpHealthProbe;
#[cfg(all(any(debug_assertions, test), not(windows)))]
pub(crate) use ownership::UnsupportedProcessOwner;
#[cfg(all(any(debug_assertions, test), windows))]
pub(crate) use ownership::WindowsJobProcessOwner;
#[cfg(any(debug_assertions, test))]
pub(crate) use planning::{RuntimeSettings, SystemSourceInspector, ToolchainPrograms};
#[cfg(any(debug_assertions, test))]
pub(crate) use registry::SqliteInstanceRegistry;

#[cfg(test)]
mod facade_tests;
#[cfg(test)]
mod tests;
