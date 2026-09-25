//! Claude Code, driven through its CLI's stream-json mode: one `claude -p` process per
//! invocation, with the control messages the official SDKs use for permission prompts, questions,
//! interrupts and `initialize`.
//!
//! - `launch`: Orchid's launch intent → CLI arguments and environment.
//! - `connection`: JSON lines to and from the supervised process.
//! - `events`: Claude messages → normalized runtime events.
//! - `requests`: permission prompts and questions ↔ Orchid's runtime requests.
//! - `runtime`: the `AgentRuntime` implementation.
//! - `discovery`: models from `initialize` and skills from the setup's folders.
mod connection;
pub mod discovery;
mod events;
pub mod launch;
mod requests;
mod runtime;

pub use runtime::ClaudeRuntime;

pub const PROVIDER: &str = "claude";

#[cfg(test)]
mod tests;
