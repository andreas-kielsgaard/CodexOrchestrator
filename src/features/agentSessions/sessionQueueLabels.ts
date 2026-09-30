import type {
  SessionQueueEntryViewDto,
  TargetOperationActionKindDto,
} from '../../application/agentSessions/sessionQueue';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';

export function queueTargetLabel(
  target: SessionExecutionSelectionDto | SessionExecutionTargetDto | null,
): { device: string; workspace: string } {
  if (!target) return { device: 'Current device', workspace: 'Current workspace' };
  if ('workspace' in target) {
    if (target.workspace.kind === 'existing')
      return { device: target.execution.deviceName, workspace: target.workspace.target.path };
    if (target.workspace.kind === 'create')
      return {
        device: target.execution.deviceName,
        workspace: `${target.workspace.branchRef.replace(/^refs\/heads\//, '')} · create worktree`,
      };
    return { device: target.execution.deviceName, workspace: 'Empty workspace' };
  }
  return { device: target.execution.deviceName, workspace: target.path };
}

export function queueEntryTitle(item: SessionQueueEntryViewDto) {
  if (item.entry.item.kind === 'compaction') return 'Compact conversation';
  if (item.message) return item.message.invocation.submittedText.split(/\r?\n/, 1)[0] || 'Message';
  const request = item.targetOperation?.request;
  if (!request) return 'Target switch';
  const workspace = request.destination.workspace;
  const branch =
    workspace.kind === 'existing'
      ? workspace.target.branchRef
      : workspace.kind === 'create'
        ? workspace.branchRef
        : '';
  return request.intent === 'continue_work'
    ? `Continue this work at ${request.destination.execution.deviceName}`
    : `Change conversation target to ${request.destination.execution.deviceName} ${branch.replace(/^refs\/heads\//, '')}`.trim();
}

export function queueEntryState(item: SessionQueueEntryViewDto) {
  if (item.targetOperation)
    return {
      pending: 'Waiting',
      staging: 'Preparing',
      staged: 'Ready',
      activating: 'Switching',
      completed: 'Completed',
      blocked: 'Blocked',
      canceled: 'Canceled',
    }[item.targetOperation.phase];
  return {
    pending: 'Queued',
    executing: 'Running',
    blocked: 'Blocked',
    completed: 'Completed',
    canceled: 'Canceled',
  }[item.entry.state];
}

const actionLabels: Record<TargetOperationActionKindDto, string> = {
  prepare_device: 'Prepare device',
  resolve_revision: 'Resolve revision',
  materialize_worktree: 'Create worktree',
  verify_worktree: 'Verify worktree',
  inspect_source: 'Inspect source',
  inspect_destination: 'Inspect destination',
  capture_snapshot: 'Capture snapshot',
  transfer_snapshot: 'Transfer snapshot',
  apply_snapshot: 'Apply snapshot',
  verify_migrated_state: 'Verify migrated state',
  activate_sister: 'Activate sister worktree',
  activate_target: 'Activate target',
};

export function targetActionLabel(kind: TargetOperationActionKindDto) {
  return actionLabels[kind];
}
