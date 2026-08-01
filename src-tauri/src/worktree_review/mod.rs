mod catalog;
pub(crate) mod comparison;
mod composition;
#[cfg(debug_assertions)]
pub(crate) mod debug_controller;
pub(crate) mod detail;
mod progress;
mod proof_evidence;
mod service;
pub(crate) mod transport;
pub(crate) mod worktree_build;

pub(crate) use composition::compose;
