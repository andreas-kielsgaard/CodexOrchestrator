import type { StartCodexTaskRunCommandResult } from '../../../capabilities/taskRunLaunch';
import { capitalize } from '../../../app/viewModels/formatting';

export function formatRunResult(result: StartCodexTaskRunCommandResult): string {
  const parts = [
    `${capitalize(result.status)} run ${result.taskRunId}`,
    `task ${result.task.executionState}`,
  ];

  if (result.taskRun.executionState !== result.task.executionState) {
    parts.push(`run ${result.taskRun.executionState}`);
  }

  if (result.exitCode !== undefined) {
    parts.push(`exit ${result.exitCode}`);
  }

  if (result.error ?? result.statusReason) {
    parts.push(result.error ?? result.statusReason ?? '');
  }

  return parts.filter(Boolean).join(' | ');
}
