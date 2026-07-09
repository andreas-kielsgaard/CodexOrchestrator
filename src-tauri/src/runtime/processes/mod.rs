//! Operating-system process supervision boundary.
//!
//! The future supervisor will own child handles and invocation-scoped cancellation here. It will
//! not own scheduling policy, Agent Session persistence, or transcript presentation.
//!
//! AS-00 intentionally provides no supervisor implementation.
