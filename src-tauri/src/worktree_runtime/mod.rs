//! Product-owned worktree test-instance identity, persistence, and process-tree ownership.
//!
//! This module does not schedule work, provision credentials, approve actions, or provide
//! pause/resume semantics. Projected configuration and observed runtime facts remain distinct.

mod application;
mod domain;
mod execution;
mod facade;
mod health;
mod ownership;
mod planning;
mod projection;
mod registry;

#[allow(unused_imports)]
pub(crate) use facade::{
    HealthState, IsolatedTestRequest, RequestedTestInstance, TestActionOutcome, TestActionResult,
    TestInstanceError, TestInstanceErrorKind, TestInstanceHandle, TestInstancePhase,
    TestInstanceStatus, TestSourceRef, WorktreeTestInstances,
};

#[cfg(test)]
mod facade_tests;
#[cfg(test)]
mod tests;
