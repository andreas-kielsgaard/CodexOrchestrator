//! Codex CLI implementation of the provider-neutral Agent Runtime port.
//!
//! Argument/capability mapping, JSONL normalization, and process coordination are intentionally
//! separate. The process supervisor remains the sole owner of child processes.

mod arguments;
mod capabilities;
mod protocol;
mod runtime;

#[cfg(test)]
pub(crate) use capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe, resolve_program};
#[allow(unused_imports)]
pub(crate) use runtime::CodexCliRuntime;

#[cfg(test)]
mod tests;
