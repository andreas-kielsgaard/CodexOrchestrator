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
  | 'activate_target';

export interface TargetOperationActionDto {
  readonly kind: TargetOperationActionKindDto;
  readonly status: 'pending' | 'running' | 'completed' | 'failed';
  readonly detail: string | null;
  readonly error: string | null;
}

export type TargetSwitchIntentDto = 'change_conversation_target' | 'continue_work';
export type MigrationStrategyDto = 'existing_sister' | 'link_existing' | 'create_linked';
export interface MigrationResolutionDto {
  readonly strategy: MigrationStrategyDto;
  readonly repositoryId: string;
  readonly branchRef: string;
}
export interface TargetSwitchRequestDto {
  readonly destination: SessionExecutionSelectionDto;
  readonly intent: TargetSwitchIntentDto;
  readonly migrationResolution: MigrationResolutionDto | null;
}
export interface TargetSwitchFactsDto {
  readonly destination: SessionExecutionTargetDto | null;
  readonly destinationReady: boolean | null;
  readonly unknown: readonly string[];
  readonly blockers: readonly string[];
}
export interface TargetSwitchProjectionDto {
  readonly calculatedAt: string;
  readonly requestRevision: number;
  readonly stale: boolean;
  readonly source: SessionExecutionTargetDto | null;
  readonly facts: TargetSwitchFactsDto;
  readonly steps: readonly TargetOperationActionDto[];
}
export const retargetRequest = (
  destination: SessionExecutionSelectionDto,
): TargetSwitchRequestDto => ({
  destination,
  intent: 'change_conversation_target',
  migrationResolution: null,
});

export interface SessionTargetOperationDto {
  readonly id: string;
  readonly sessionId: string;
  readonly sourceTarget: SessionExecutionTargetDto | null;
  readonly request: TargetSwitchRequestDto;
  readonly needsResolution: boolean;
  readonly receipt: import('./sessionHistory').WorkspaceTransitionReceiptDto | null;
  readonly projectedPlan: TargetSwitchProjectionDto;
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
    | { readonly kind: 'compaction'; readonly invocationId: string }
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
  readonly request: TargetSwitchRequestDto;
}

export interface SessionQueueClient {
  loadQueue(sessionId: string): Promise<SessionQueueViewDto>;
  enqueueTargetOperation(input: {
    readonly sessionId: string;
    readonly request: TargetSwitchRequestDto;
    readonly beforeEntryId?: string;
  }): Promise<SessionQueueViewDto>;
  enqueueCompaction(sessionId: string): Promise<SessionQueueViewDto>;
  reorderQueue(input: {
    readonly sessionId: string;
    readonly expectedRevision: number;
    readonly entryIds: readonly string[];
  }): Promise<SessionQueueViewDto>;
  updateQueuedMessage(input: UpdateQueuedMessageInputDto): Promise<SessionQueueViewDto>;
  updateTargetOperation(input: UpdateTargetOperationInputDto): Promise<SessionQueueViewDto>;
  recalculateTargetProjection(input: {
    sessionId: string;
    entryId: string;
    expectedRevision: number;
  }): Promise<SessionQueueViewDto>;
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
      target = item.targetOperation.request.destination;
    return { item, target };
  });
}

export function projectedQueueSource(
  queue: SessionQueueViewDto | null | undefined,
  beforeEntryId?: string,
): SessionExecutionTargetDto | null {
  if (!queue) return null;
  let source = queue.activeTarget;
  for (const item of queue.entries) {
    if (item.entry.id === beforeEntryId) break;
    if (
      !item.targetOperation ||
      item.entry.state === 'completed' ||
      item.entry.state === 'canceled'
    )
      continue;
    const operation = item.targetOperation;
    source =
      operation.stagedTarget ??
      (operation.request.destination.workspace.kind === 'existing'
        ? operation.request.destination.workspace.target
        : null);
  }
  return source;
}
