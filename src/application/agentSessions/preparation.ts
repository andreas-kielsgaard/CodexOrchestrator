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

export function isSessionPreparing(preparation: SessionPreparationDto | null | undefined): boolean {
  return (
    preparation?.phase === 'accepted' ||
    preparation?.phase === 'preparing' ||
    (preparation?.phase === 'ready' &&
      preparation.steps.some((step) => step.id === 'delivery' && step.status !== 'completed'))
  );
}
