import type { SessionExecutionTargetDto } from '../executionTargets/contracts';
import type { TargetSwitchIntentDto, TargetOperationActionDto } from './sessionQueue';
export interface WorkspaceTransitionReceiptDto {
  readonly operationId: string;
  readonly intent: TargetSwitchIntentDto;
  readonly source: SessionExecutionTargetDto | null;
  readonly destination: SessionExecutionTargetDto;
  readonly actions: readonly TargetOperationActionDto[];
  readonly completedAt: string;
}
export interface SessionTimelineDto {
  readonly positions: readonly {
    ordinal: number;
    invocationId: string | null;
    operationId: string | null;
  }[];
  readonly transitions: readonly WorkspaceTransitionReceiptDto[];
  readonly providers: readonly { invocationId: string; provider: string }[];
}
