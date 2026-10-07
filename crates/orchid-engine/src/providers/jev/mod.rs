//! Jev typed API evaluations, using the shared Agent Session runtime lifecycle.
mod client;
pub mod contracts;
mod runtime;

pub use runtime::JevRuntime;
pub const PROVIDER: &str = "jev";
pub const DEFAULT_MODEL: &str = "jev-latest";
