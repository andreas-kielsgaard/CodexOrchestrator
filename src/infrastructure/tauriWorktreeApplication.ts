import { invoke } from '@tauri-apps/api/core';
import type {
  GitCreateWorktreeInput,
  GitCreateWorktreeResult,
  GitWorktreeCreator,
} from '../application/taskWorktreeSelection';

export type WorktreeApplicationInvoke = <Result>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<Result>;

interface PhysicalWorktreeResult {
  readonly repositoryRoot: string;
  readonly worktreeRoot: string;
  readonly commitId: string;
  readonly headRef?: string;
}

/** Native adapter for the single product-owned physical worktree mutation boundary. */
export function createTauriWorktreeCreator(
  invokeCommand: WorktreeApplicationInvoke = invoke,
): GitWorktreeCreator {
  return {
    async createWorktree(input: GitCreateWorktreeInput): Promise<GitCreateWorktreeResult> {
      const result = await invokeCommand<PhysicalWorktreeResult>('create_physical_worktree', {
        input: {
          repositoryRoot: input.repoRootPath,
          worktreeRoot: input.worktreePath,
          branchName: input.branchName,
          ...(input.baseBranch === undefined ? {} : { startPointRef: input.baseBranch }),
        },
      });
      return {
        repoRootPath: result.repositoryRoot,
        worktreePath: result.worktreeRoot,
        branchName: input.branchName,
        ...(input.baseBranch === undefined ? {} : { baseBranch: input.baseBranch }),
      };
    },
  };
}

export const tauriWorktreeCreator = createTauriWorktreeCreator();
