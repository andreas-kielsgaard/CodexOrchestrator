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
  WorktreeReviewOverview,
} from './contracts';

/** Product use cases exposed to the Worktree Review presentation. */
export interface WorktreeReviewClient {
  overview(): Promise<WorktreeReviewOverview>;
  connectRepository?(repositoryRoot: string): Promise<WorktreeReviewOverview>;
  selectRepository(repositoryId: RepositoryId): Promise<WorktreeReviewOverview>;
  branchDetail(repositoryId: RepositoryId, branchRef: BranchRef): Promise<BranchReviewDetail>;
  branchHistory(
    repositoryId: RepositoryId,
    branchRef: BranchRef,
    cursor?: string,
  ): Promise<BranchHistoryPage>;
  associateWorktree(input: AssociateWorktreeRequest): Promise<AssociatedWorktree>;
  createWorktree(input: CreateWorktreeRequest): Promise<AssociatedWorktree>;
  createBuild(input: CreateBuildRequest): Promise<ReviewBuild>;
  openBuild(input: OpenBuildRequest): Promise<void>;
}
