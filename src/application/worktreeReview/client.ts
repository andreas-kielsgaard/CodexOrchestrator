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
  WorktreeReviewOverview,
} from './contracts';

/** Product use cases exposed to the Worktree Review presentation. */
export interface WorktreeReviewClient {
  overview(): Promise<WorktreeReviewOverview>;
  selectRepository(repositoryId: RepositoryId): Promise<WorktreeReviewOverview>;
  targetDetail(target: ReviewTarget): Promise<BranchReviewDetail>;
  commitHistory(query: CommitHistoryQuery, cursor?: string): Promise<CommitHistoryPage>;
  branchGraph(
    repositoryId: RepositoryId,
    limit?: number,
    snapshotId?: string,
  ): Promise<BranchGraphData>;
  worktreeActivity(
    repositoryId: RepositoryId,
    worktreeIds: readonly WorktreeId[],
  ): Promise<readonly WorktreeActivity[]>;
  associateWorktree(input: AssociateWorktreeRequest): Promise<AssociatedWorktree>;
  createWorktree(input: CreateWorktreeRequest): Promise<AssociatedWorktree>;
  createBuild(input: CreateBuildRequest): Promise<ReviewBuild>;
  openBuild(input: OpenBuildRequest): Promise<void>;
}
