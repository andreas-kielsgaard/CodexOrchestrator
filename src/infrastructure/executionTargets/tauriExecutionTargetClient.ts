import { invoke } from '@tauri-apps/api/core';
import type { ExecutionTargetClient } from '../../application/executionTargets/contracts';
export function createTauriExecutionTargetClient(
  invokeCommand: typeof invoke = invoke,
): ExecutionTargetClient {
  return {
    resolvePublishedTip: (input) => invokeCommand('resolve_published_worktree_commit', { input }),
    listDevices: () => invokeCommand('list_execution_target_devices'),
    listWorktreeChoices: (scope) => invokeCommand('list_execution_worktree_choices', { scope }),
    listTargets: (repositoryId, branchRef) =>
      invokeCommand('list_session_execution_targets', { input: { repositoryId, branchRef } }),
    loadRuntime: (execution, workingDirectory) =>
      invokeCommand('load_execution_target_runtime', {
        input: { execution, ...(workingDirectory ? { workingDirectory } : {}) },
      }),
    listRepositoryLocations: () => invokeCommand('list_repository_device_locations'),
    saveRepositoryLocation: (input) => invokeCommand('save_repository_device_location', { input }),
  };
}
export const tauriExecutionTargetClient = createTauriExecutionTargetClient();
