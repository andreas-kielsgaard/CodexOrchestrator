//! Durable Epic Plan Proposal and initiation facts. Execution, scheduling, and acceptance remain out of scope.

pub(crate) mod application;
pub(crate) mod bootstrap_transition;
pub(crate) mod confirmation;
pub(crate) mod conversation_harness;
pub(crate) mod conversation_harness_revision;
pub(crate) mod conversation_harness_working_copy;
pub(crate) mod domain;
pub(crate) mod file_review_git_producer;
pub(crate) mod file_review_originating_entry;
pub(crate) mod initiated_sprint_git_authority;
pub(crate) mod mcp;
pub(crate) mod repository;
pub(crate) mod transport;
