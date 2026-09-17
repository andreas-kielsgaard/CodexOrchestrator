import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../executionTargets/contracts';
import type {
  DirectUserInvocationResolutionDto,
  SessionCreationResolutionDto,
  SandboxModeDto,
} from '../executionConfiguration';
import type { SessionFolderTarget } from './organization';

export interface SessionPreparationStepDto {
  readonly id: string;
  readonly label: string;
  readonly status: 'pending' | 'running' | 'completed' | 'failed';
  readonly error?: string | null;
}
export interface SessionPreparationDto {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly phase: 'accepted' | 'preparing' | 'ready' | 'failed' | 'canceled';
  readonly selection: SessionExecutionSelectionDto | null;
  readonly steps: readonly SessionPreparationStepDto[];
  readonly error: string | null;
  readonly canRetry: boolean;
  readonly resolvedTarget?: SessionExecutionTargetDto | null;
  readonly resolution?: DirectUserInvocationResolutionDto | null;
  readonly currentResolution?: SessionCreationResolutionDto | null;
}
export interface SendPreparedAgentSessionMessageInput {
  readonly sessionId: string | null;
  readonly submissionId: string;
  readonly submittedText: string;
  readonly title: string | null;
  readonly workingDirectory: string | null;
  readonly executionSelection: SessionExecutionSelectionDto | null;
  readonly model: string | null;
  readonly reasoningMode: string | null;
  readonly sandboxMode?: SandboxModeDto | null;
  readonly folderTarget?: SessionFolderTarget | null;
}

/**
 * A device/worktree change owned by the Session, rather than by one submitted prompt.
 * Pending and running transitions keep a later prompt behind their worktree work.
 */
export type SessionTargetTransitionPhaseDto =
  | 'pending'
  | 'running'
  | 'ready'
  | 'failed'
  | 'canceled';

export interface SessionTargetTransitionTaskDto {
  readonly kind:
    | 'inspect_source'
    | 'inspect_destination'
    | 'capture_snapshot'
    | 'materialize_destination'
    | 'transfer_snapshot'
    | 'apply_snapshot'
    | 'verify_destination'
    | 'activate_sister';
  readonly status: 'pending' | 'running' | 'completed' | 'failed';
  readonly detail?: string | null;
  readonly error?: string | null;
}

export interface SessionTargetTransitionSnapshotDto {
  readonly sourceHead: string | null;
  readonly destinationHead: string | null;
  readonly commitBundle: SessionTargetTransitionSnapshotArtifactDto;
  readonly stagedPatch: SessionTargetTransitionSnapshotArtifactDto;
  readonly unstagedPatch: SessionTargetTransitionSnapshotArtifactDto;
  readonly untrackedFiles: SessionTargetTransitionSnapshotArtifactDto;
  readonly totalBytes: number;
}

export interface SessionTargetTransitionSnapshotArtifactDto {
  readonly entryCount: number;
  readonly bytes: number;
  readonly digest?: string | null;
}

export interface SessionTargetTransitionEstimateDto {
  readonly snapshotBytes: number;
  readonly bytesPerSecond: number;
  readonly estimatedSeconds: number;
  readonly measuredAt: string;
}

export interface SessionTargetTransitionDto {
  readonly sessionId: string;
  readonly sourceTarget: SessionExecutionTargetDto;
  readonly destinationSelection: SessionExecutionSelectionDto;
  readonly phase: SessionTargetTransitionPhaseDto;
  readonly tasks: readonly SessionTargetTransitionTaskDto[];
  readonly snapshot?: SessionTargetTransitionSnapshotDto | null;
  readonly transferEstimate?: SessionTargetTransitionEstimateDto | null;
  readonly resolvedTarget?: SessionExecutionTargetDto | null;
  readonly error?: string | null;
}

export interface RequestSessionTargetTransitionInput {
  readonly sessionId: string;
  readonly sourceTarget: SessionExecutionTargetDto;
  readonly destinationSelection: SessionExecutionSelectionDto;
}

export function isSessionTargetTransitionRunning(
  transition: SessionTargetTransitionDto | null | undefined,
): boolean {
  return transition?.phase === 'running';
}

export function isSessionPreparing(preparation: SessionPreparationDto | null | undefined): boolean {
  return (
    preparation?.phase === 'accepted' ||
    preparation?.phase === 'preparing' ||
    (preparation?.phase === 'ready' &&
      preparation.steps.some((step) => step.id === 'delivery' && step.status !== 'completed'))
  );
}
