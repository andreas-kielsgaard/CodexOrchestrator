import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import type {
  MigrationStrategyDto,
  TargetSwitchIntentDto,
  TargetSwitchRequestDto,
} from '../../application/agentSessions/sessionQueue';

export function targetSwitchRequest(
  destination: SessionExecutionSelectionDto,
  source: SessionExecutionTargetDto | null,
  intent: TargetSwitchIntentDto,
  strategy: MigrationStrategyDto | '',
): TargetSwitchRequestDto | null {
  if (intent === 'change_conversation_target')
    return { destination, intent, migrationResolution: null };
  const workspace = destination.workspace;
  const repositoryId =
    workspace.kind === 'existing'
      ? workspace.target.repositoryId
      : workspace.kind === 'create'
        ? workspace.repositoryId
        : '';
  const branchRef =
    workspace.kind === 'existing'
      ? workspace.target.branchRef
      : workspace.kind === 'create'
        ? workspace.branchRef
        : '';
  if (
    !source ||
    !strategy ||
    !repositoryId ||
    source.repositoryId !== repositoryId ||
    source.branchRef !== branchRef ||
    (workspace.kind === 'create') !== (strategy === 'create_linked')
  )
    return null;
  return { destination, intent, migrationResolution: { strategy, repositoryId, branchRef } };
}
