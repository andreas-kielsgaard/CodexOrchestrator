//! Agent Session backend boundary.
//!
//! Later slices add Agent Session application coordination and persistence here. This module may
//! depend on runtime ports, but provider-specific Codex protocol and operating-system process
//! ownership stay under `runtime`.
//!
//! AS-00 intentionally defines no session records, schema, commands, or behavior.
