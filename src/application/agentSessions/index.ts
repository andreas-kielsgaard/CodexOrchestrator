/**
 * Serializable Agent Session client boundary.
 *
 * Rust owns lifecycle validation and durable mutation policy. These DTOs describe transport-safe
 * values consumed by frontend application and feature code; they intentionally contain no React,
 * Tauri, SQLite, process, or provider-protocol types.
 */
export * from './contracts';
export * from './agentIdentity';
