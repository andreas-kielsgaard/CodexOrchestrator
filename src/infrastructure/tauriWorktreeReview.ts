import { invoke } from '@tauri-apps/api/core';
import type {
  AssociateWorktreeRequest,
  AssociatedWorktree,
  ReviewTarget,
  CommitHistoryQuery,
  CommitHistoryPage,
  BranchGraphData,
  WorktreeId,
  WorktreeActivity,
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
    selectRepository: (repositoryId: RepositoryId) =>
      invokeCommand<WorktreeReviewOverview>('select_worktree_review_repository', {
        input: { repositoryId },
      }),
    targetDetail: (input: ReviewTarget) =>
      invokeCommand<BranchReviewDetail>('worktree_review_target_detail', { input }),
    commitHistory: (query: CommitHistoryQuery, cursor?: string) =>
      invokeCommand<CommitHistoryPage>('worktree_review_commit_history', {
        input: { query, ...(cursor ? { cursor } : {}) },
      }),
    branchGraph: (repositoryId: RepositoryId, limit = 1200, snapshotId?: string) =>
      invokeCommand<BranchGraphData>('worktree_review_branch_graph', {
        input: { repositoryId, limit, snapshotId },
      }),
    worktreeActivity: (repositoryId: RepositoryId, worktreeIds: readonly WorktreeId[]) =>
      invokeCommand<readonly WorktreeActivity[]>('worktree_review_worktree_activity', {
        input: { repositoryId, worktreeIds },
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
