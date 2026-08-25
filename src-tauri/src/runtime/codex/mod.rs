//! Codex CLI implementation of the provider-neutral Agent Runtime port.
//!
//! Argument/capability mapping, JSONL normalization, and process coordination are intentionally
//! separate. The process supervisor remains the sole owner of child processes.

mod arguments;
mod capabilities;
mod protocol;
mod runtime;

pub(crate) use capabilities::resolve_program;
#[cfg(all(test, feature = "live-tests"))]
pub(crate) use capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe};
#[allow(unused_imports)]
pub(crate) use runtime::CodexCliRuntime;

#[cfg(test)]
mod tests;
