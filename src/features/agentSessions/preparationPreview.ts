import type { SessionPreparationStepDto } from '../../application/agentSessions/preparation';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
export function preparationPreview(
  selection: SessionExecutionSelectionDto | null,
  current: SessionExecutionTargetDto | null,
): readonly SessionPreparationStepDto[] {
  if (!selection) return [];
  const steps: SessionPreparationStepDto[] = [];
  const add = (id: string, label: string) => steps.push({ id, label, status: 'pending' });
  const changedStore =
    current &&
    (current.execution.deviceId !== selection.execution.deviceId ||
      current.execution.configurationRef !== selection.execution.configurationRef);
  if (selection.execution.connection.kind === 'ssh' && (!current || changedStore))
    add('connect', `Connect to ${selection.execution.deviceName}`);
  if (selection.workspace.kind === 'create')
    add('workspace', `Create worktree at ${selection.workspace.commit.slice(0, 12)}`);
  else if (
    selection.workspace.kind === 'existing' &&
    selection.workspace.target.worktreeId !== current?.worktreeId
  )
    add('workspace', 'Resolve selected worktree');
  else if (selection.workspace.kind === 'auxiliary') add('workspace', 'Resolve working folder');
  if (changedStore) add('history', 'Copy conversation history');
  add('conversation', current ? 'Resume conversation' : 'Start conversation');
  add('delivery', 'Deliver submitted prompt');
  return steps;
}
