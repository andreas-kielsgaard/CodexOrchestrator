import type { RepositoryId } from '../repositoryCatalog/contracts';
export type OpaqueId<Kind extends string> = string & { readonly __kind?: Kind };

export type { RepositoryId } from '../repositoryCatalog/contracts';
export type BranchRef = OpaqueId<'branch-ref'>;
export type WorktreeId = OpaqueId<'worktree'>;
export type GitObjectId = OpaqueId<'git-object'>;

export interface GitCommit {
  readonly objectId: GitObjectId;
  readonly abbreviatedObjectId: string;
  readonly subject: string;
  readonly author: string;
  readonly committedAt: string;
}

export type CommitSourceContext =
  | { readonly kind: 'branch'; readonly branchRef: BranchRef; readonly tipObjectId: GitObjectId }
  | {
      readonly kind: 'worktree';
      readonly worktreeId: WorktreeId;
      readonly tipObjectId: GitObjectId;
    };

export type ReviewTarget =
  | { readonly kind: 'branch'; readonly repositoryId: RepositoryId; readonly branchRef: BranchRef }
  | {
      readonly kind: 'worktree';
      readonly repositoryId: RepositoryId;
      readonly worktreeId: WorktreeId;
    }
  | {
      readonly kind: 'commit';
      readonly repositoryId: RepositoryId;
      readonly objectId: GitObjectId;
      readonly context: CommitSourceContext;
    };

export interface ActivityEstimate {
  readonly changedAt: string;
  readonly observedAt: string;
  readonly basis: 'commit_fallback' | 'changed_file_estimate' | 'changed_file_and_index_estimate';
  readonly stagedFiles: number;
  readonly unstagedFiles: number;
  readonly untrackedFiles: number;
}
export interface WorktreeActivity {
  readonly worktreeId: WorktreeId;
  readonly headObjectId: GitObjectId;
  readonly estimate: ActivityEstimate;
}

export interface ReviewBranch {
  readonly target: ReviewTarget;
  readonly availableWorktreeCount: number;
  readonly worktreeIds: readonly WorktreeId[];
  readonly activity: ActivityEstimate | null;
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef | null;
  readonly displayName: string;
  readonly tip: GitCommit;
  readonly aheadOfDefault: number;
  readonly behindDefault: number;
  readonly associatedWorktreeCount: number;
}

export type HistoryScope =
  | {
      readonly kind: 'ancestry';
      readonly tipObjectId: GitObjectId;
      readonly excludedBaseObjectId?: GitObjectId | null;
    }
  | {
      readonly kind: 'graph_range';
      readonly snapshotId: string;
      readonly rangeId: string;
    };
export interface CommitHistoryQuery {
  readonly target: ReviewTarget;
  readonly scope: HistoryScope;
}
export interface CommitHistoryPage {
  readonly scope: HistoryScope;
  readonly totalCount: number;
  readonly commits: readonly GitCommit[];
  readonly nextCursor: string | null;
}
export interface GraphAnchor {
  readonly objectId: GitObjectId;
  readonly parentIds: readonly GitObjectId[];
  readonly boundary: boolean;
  readonly merge: boolean;
}
export interface GraphConnection {
  readonly id: string;
  readonly from: GitObjectId;
  readonly to: GitObjectId;
  readonly commitCount: number;
  readonly collapsed: boolean;
  readonly incomplete: boolean;
  readonly eligibleSources: readonly ReviewTarget[];
  readonly scope: Extract<HistoryScope, { kind: 'graph_range' }>;
}
export interface BranchGraphData {
  readonly snapshotId: string;
  readonly referenceTarget: ReviewTarget | null;
  readonly targets: readonly ReviewBranch[];
  readonly anchors: readonly GraphAnchor[];
  readonly connections: readonly GraphConnection[];
  readonly hasMore: boolean;
  readonly loadedCommitCount: number;
}

export interface BranchReadSource {
  branchGraph(
    repositoryId: RepositoryId,
    limit?: number,
    snapshotId?: string,
  ): Promise<BranchGraphData>;
}

export interface RepositoryBranchSource extends BranchReadSource {
  listRepositories(): Promise<
    readonly import('../repositoryCatalog/contracts').RegisteredRepository[]
  >;
}
