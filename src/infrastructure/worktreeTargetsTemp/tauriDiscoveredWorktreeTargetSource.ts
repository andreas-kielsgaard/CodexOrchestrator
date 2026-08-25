import { invoke } from '@tauri-apps/api/core';
import type {
  RepoBranchWorktreeTargetSource,
  ResolvedRepoBranchWorktreeTarget,
} from '../../application/worktreeTargets';

export type DiscoveredWorktreeTargetInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createTauriDiscoveredWorktreeTargetSource(
  invokeCommand: DiscoveredWorktreeTargetInvoke = invoke,
): RepoBranchWorktreeTargetSource {
  return {
    listTargets: () =>
      invokeCommand<ResolvedRepoBranchWorktreeTarget[]>('list_discovered_worktree_targets'),
  };
}

export const tauriDiscoveredWorktreeTargetSource = createTauriDiscoveredWorktreeTargetSource();
