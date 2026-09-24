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
  DetachedWorktreeDetail,
  CreateBuildRequest,
  CreateWorktreeRequest,
  OpenBuildRequest,
  OpenBuildOutcome,
  RepositoryId,
  ReviewBuild,
  BuildId,
  BuildLogChunk,
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
  detachedWorktrees(repositoryId: RepositoryId): Promise<readonly DetachedWorktreeDetail[]>;
  associateWorktree(input: AssociateWorktreeRequest): Promise<AssociatedWorktree>;
  createWorktree(input: CreateWorktreeRequest): Promise<AssociatedWorktree>;
  createBuild(input: CreateBuildRequest): Promise<ReviewBuild>;
  readBuildLog(buildId: BuildId, attemptId: string, offset: number): Promise<BuildLogChunk>;
  openBuild(input: OpenBuildRequest): Promise<OpenBuildOutcome>;
}
