//! Orchid control tools shared by desktop and execution hosts.
pub mod mcp;
mod service;
pub use service::{required, tool_map, OrchidControl};
