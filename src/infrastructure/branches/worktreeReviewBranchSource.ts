import type { RepositoryBranchSource } from '../../application/branches';
import type { WorktreeReviewClient } from '../../application/worktreeReview';
/** Existing read-only Git queries, without review actions or selection state. */
export function worktreeReviewBranchSource(client: WorktreeReviewClient): RepositoryBranchSource {
  return {
    listRepositories: async () => (await client.overview()).repositories,
    branchGraph: (repositoryId, limit, snapshotId) =>
      client.branchGraph(repositoryId, limit, snapshotId),
  };
}
