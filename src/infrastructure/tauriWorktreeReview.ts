import { invoke } from '@tauri-apps/api/core';
import type {
  AssociateWorktreeRequest,
  AssociatedWorktree,
  BranchRef,
  BranchHistoryPage,
  BranchReviewDetail,
  CreateBuildRequest,
  CreateWorktreeRequest,
  OpenBuildRequest,
  RepositoryId,
  ReviewBuild,
  WorktreeReviewClient,
  WorktreeReviewOverview,
} from '../application/worktreeReview';

export type WorktreeReviewInvoke = <Result>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<Result>;

/** Maps the product use cases one-to-one onto the typed native transport. */
export function createTauriWorktreeReviewClient(
  invokeCommand: WorktreeReviewInvoke = invoke,
): WorktreeReviewClient {
  return {
    overview: () => invokeCommand<WorktreeReviewOverview>('worktree_review_overview'),
    connectRepository: (repositoryRoot: string) =>
      invokeCommand<WorktreeReviewOverview>('connect_worktree_review_repository', {
        input: { repositoryRoot },
      }),
    selectRepository: (repositoryId: RepositoryId) =>
      invokeCommand<WorktreeReviewOverview>('select_worktree_review_repository', {
        input: { repositoryId },
      }),
    branchDetail: (repositoryId: RepositoryId, branchRef: BranchRef) =>
      invokeCommand<BranchReviewDetail>('worktree_review_branch_detail', {
        input: { repositoryId, branchRef },
      }),
    branchHistory: (repositoryId: RepositoryId, branchRef: BranchRef, cursor?: string) =>
      invokeCommand<BranchHistoryPage>('worktree_review_branch_history', {
        input: { repositoryId, branchRef, ...(cursor ? { cursor } : {}) },
      }),
    associateWorktree: (input: AssociateWorktreeRequest) =>
      invokeCommand<AssociatedWorktree>('associate_worktree_review_worktree', { input }),
    createWorktree: (input: CreateWorktreeRequest) =>
      invokeCommand<AssociatedWorktree>('create_worktree_review_worktree', { input }),
    createBuild: (input: CreateBuildRequest) =>
      invokeCommand<ReviewBuild>('create_worktree_review_build', { input }),
    openBuild: (input: OpenBuildRequest) =>
      invokeCommand<void>('worktree_review_open_build', { input }),
  };
}

export const tauriWorktreeReview = createTauriWorktreeReviewClient();
