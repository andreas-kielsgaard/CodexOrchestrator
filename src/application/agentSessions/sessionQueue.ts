import type { AgentInvocationDto } from './contracts';
import type { SessionPreparationDto } from './preparation';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../executionTargets/contracts';
import type { SandboxModeDto } from '../executionConfiguration';

export type SessionQueueEntryStateDto =
  'pending' | 'executing' | 'blocked' | 'completed' | 'canceled';

export type TargetOperationPhaseDto =
  'pending' | 'staging' | 'staged' | 'activating' | 'completed' | 'blocked' | 'canceled';

export type TargetOperationActionKindDto =
  | 'prepare_device'
  | 'resolve_revision'
  | 'materialize_worktree'
  | 'verify_worktree'
  | 'inspect_source'
  | 'inspect_destination'
  | 'capture_snapshot'
  | 'transfer_snapshot'
  | 'apply_snapshot'
  | 'verify_migrated_state'
  | 'activate_sister'
  | 'select_provider_context'
  | 'activate_target';

export interface TargetOperationActionDto {
  readonly kind: TargetOperationActionKindDto;
  readonly status: 'pending' | 'running' | 'completed' | 'failed';
  readonly detail: string | null;
  readonly error: string | null;
}

export interface SessionTargetOperationDto {
  readonly id: string;
  readonly sessionId: string;
  readonly sourceTarget: SessionExecutionTargetDto | null;
  readonly destinationSelection: SessionExecutionSelectionDto;
  readonly phase: TargetOperationPhaseDto;
  readonly actions: readonly TargetOperationActionDto[];
  readonly stagedTarget: SessionExecutionTargetDto | null;
  readonly error: string | null;
  readonly generation: number;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface SessionQueueEntryDto {
  readonly id: string;
  readonly sessionId: string;
  readonly ordinal: number;
  readonly state: SessionQueueEntryStateDto;
  readonly item:
    | { readonly kind: 'message'; readonly invocationId: string }
    | { readonly kind: 'target_operation'; readonly operationId: string };
  readonly blockedReason: string | null;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface SessionQueueEntryViewDto {
  readonly entry: SessionQueueEntryDto;
  readonly message: {
    readonly invocation: AgentInvocationDto;
    readonly preparation: SessionPreparationDto;
  } | null;
  readonly targetOperation: SessionTargetOperationDto | null;
}

export interface SessionQueueViewDto {
  readonly sessionId: string;
  readonly revision: number;
  readonly activeTarget: SessionExecutionTargetDto | null;
  readonly entries: readonly SessionQueueEntryViewDto[];
}

export interface UpdateQueuedMessageInputDto {
  readonly sessionId: string;
  readonly entryId: string;
  readonly expectedRevision: number;
  readonly submittedText: string;
  readonly model: string | null;
  readonly reasoningMode: string | null;
  readonly sandboxMode?: SandboxModeDto | null;
}

export interface UpdateTargetOperationInputDto {
  readonly sessionId: string;
  readonly entryId: string;
  readonly expectedRevision: number;
  readonly destination: SessionExecutionSelectionDto;
}

export interface SessionQueueClient {
  loadQueue(sessionId: string): Promise<SessionQueueViewDto>;
  enqueueTargetOperation(input: {
    readonly sessionId: string;
    readonly destination: SessionExecutionSelectionDto;
  }): Promise<SessionQueueViewDto>;
  reorderQueue(input: {
    readonly sessionId: string;
    readonly expectedRevision: number;
    readonly entryIds: readonly string[];
  }): Promise<SessionQueueViewDto>;
  updateQueuedMessage(input: UpdateQueuedMessageInputDto): Promise<SessionQueueViewDto>;
  updateTargetOperation(input: UpdateTargetOperationInputDto): Promise<SessionQueueViewDto>;
  startQueueEntry(sessionId: string, entryId: string): Promise<void>;
  retryQueueEntry(sessionId: string, entryId: string): Promise<void>;
  cancelQueueEntry(sessionId: string, entryId: string): Promise<void>;
}

export interface ProjectedQueueEntry {
  readonly item: SessionQueueEntryViewDto;
  readonly target: SessionExecutionSelectionDto | SessionExecutionTargetDto | null;
}

export function projectSessionQueueTargets(
  queue: SessionQueueViewDto,
): readonly ProjectedQueueEntry[] {
  let target: SessionExecutionSelectionDto | SessionExecutionTargetDto | null = queue.activeTarget;
  return queue.entries.map((item) => {
    if (item.targetOperation && item.entry.state !== 'completed' && item.entry.state !== 'canceled')
      target = item.targetOperation.destinationSelection;
    return { item, target };
  });
}
