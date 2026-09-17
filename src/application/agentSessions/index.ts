/**
 * Serializable Agent Session client boundary.
 *
 * Rust owns lifecycle validation and durable mutation policy. These DTOs describe transport-safe
 * values consumed by frontend application and feature code; they intentionally contain no React,
 * Tauri, SQLite, process, or provider-protocol types.
 */
export * from './contracts';

export * from './selections';

export type {
  RequestSessionTargetTransitionInput,
  SessionTargetTransitionDto,
  SessionTargetTransitionEstimateDto,
  SessionTargetTransitionPhaseDto,
  SessionTargetTransitionSnapshotDto,
  SessionTargetTransitionSnapshotArtifactDto,
  SessionTargetTransitionTaskDto,
} from './preparation';

export type {
  AgentSessionImportClient,
  CodexImportPreview,
  CodexImportCommand,
} from './importContracts';
