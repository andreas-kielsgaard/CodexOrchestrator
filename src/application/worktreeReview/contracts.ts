import type { RepositoryId } from '../repositoryCatalog/contracts';

export type OpaqueId<Kind extends string> = string & { readonly __kind?: Kind };

export type { RepositoryId } from '../repositoryCatalog/contracts';
export type BranchRef = OpaqueId<'branch-ref'>;
export type WorktreeId = OpaqueId<'worktree'>;
export type WorktreeAssociationId = OpaqueId<'worktree-association'>;
export type BuildId = OpaqueId<'review-build'>;
export type BuildAttemptId = OpaqueId<'build-attempt'>;
export type BuildOutputId = OpaqueId<'build-output'>;
export type CleanupJobId = OpaqueId<'cleanup-job'>;
export type CleanupReceiptId = OpaqueId<'cleanup-receipt'>;
export type GitObjectId = OpaqueId<'git-object'>;

export interface GitCommit {
  readonly objectId: GitObjectId;
  readonly abbreviatedObjectId: string;
  readonly subject: string;
  readonly author: string;
  readonly committedAt: string;
}

export type CapabilityAvailability =
  { readonly state: 'available' } | { readonly state: 'unavailable'; readonly reason: string };

export interface RepositoryReadiness {
  readonly state: 'ready' | 'degraded' | 'unavailable';
  readonly browse: CapabilityAvailability;
  readonly createWorktree: CapabilityAvailability;
  readonly build: CapabilityAvailability;
  readonly buildOutputStorage: CapabilityAvailability;
}

export interface ReviewRepository {
  readonly repositoryId: RepositoryId;
  readonly name: string;
  /** Presentation-only. Never use this path as repository identity. */
  readonly locationLabel: string;
  readonly readiness: RepositoryReadiness;
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

export type AssociationBaseline =
  | { readonly kind: 'created_at'; readonly commit: GitCommit }
  | { readonly kind: 'observed_at_association'; readonly commit: GitCommit }
  | { readonly kind: 'user_selected'; readonly commit: GitCommit };

export interface WorktreeChanges {
  readonly commitsAheadOfBaseline: number;
  readonly commitsBehindBaseline: number;
  readonly stagedFiles: number;
  readonly unstagedFiles: number;
  readonly untrackedFiles: number;
}

export type WorktreeAvailability =
  | { readonly state: 'available' }
  | { readonly state: 'missing'; readonly detail: string }
  | { readonly state: 'branch_mismatch'; readonly detail: string };

export interface AssociatedWorktree {
  readonly associationId: WorktreeAssociationId | null;
  readonly worktreeId: WorktreeId;
  readonly branchRef: BranchRef | null;
  readonly name: string;
  /** Presentation-only. Durable operations use worktreeId and associationId. */
  readonly locationLabel: string;
  readonly provenance:
    | 'git_branch_checkout'
    | 'user_associated_detached_checkout'
    | 'worktree_review_created'
    | 'physical_worktree';
  readonly ownership: 'borrowed_external' | 'managed_branch_worktree' | 'owned_build_worktree';
  readonly baseline: AssociationBaseline;
  readonly currentHead: GitCommit;
  readonly changes: WorktreeChanges;
  readonly detachedHead: boolean;
  readonly branchReachability: 'reachable' | 'not_reachable' | 'unknown' | 'unassociated';
  readonly availability: WorktreeAvailability;
}

export interface WorktreeAssociationCandidate {
  readonly worktreeId: WorktreeId;
  readonly name: string;
  readonly locationLabel: string;
  readonly currentHead: GitCommit;
  readonly detachedHead: boolean;
  readonly associationReason: string;
}

export interface BranchReviewDetail {
  readonly branch: ReviewBranch;
  readonly worktrees: readonly AssociatedWorktree[];
  readonly associationCandidates: readonly WorktreeAssociationCandidate[];
  readonly builds: readonly ReviewBuild[];
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

export type CreateBuildSource =
  | {
      readonly kind: 'physical_worktree';
      readonly worktreeId: WorktreeId;
      readonly headObjectId: GitObjectId;
      readonly snapshot: boolean;
    }
  | {
      readonly kind: 'exact_commit';
      readonly objectId: GitObjectId;
      readonly context: CommitSourceContext;
    }
  | {
      readonly kind: 'existing_worktree';
      readonly associationId: WorktreeAssociationId;
    }
  | {
      readonly kind: 'worktree_snapshot';
      readonly associationId: WorktreeAssociationId;
    }
  | {
      readonly kind: 'branch_commit';
      readonly branchRef: BranchRef;
      readonly objectId: GitObjectId;
    };

export type ReviewBuildSource =
  | {
      readonly kind: 'physical_worktree';
      readonly worktreeId: WorktreeId;
      readonly headObjectId: GitObjectId;
      readonly capturedObjectId: GitObjectId;
      readonly snapshot: boolean;
    }
  | { readonly kind: 'exact_commit'; readonly objectId: GitObjectId }
  | {
      readonly kind: 'existing_worktree';
      readonly associationId: WorktreeAssociationId;
      readonly triggerHeadObjectId: GitObjectId;
      readonly triggerVirtualCommitId?: GitObjectId;
    }
  | {
      readonly kind: 'worktree_snapshot';
      readonly associationId: WorktreeAssociationId;
      readonly headObjectId: GitObjectId;
      readonly capturedObjectId: GitObjectId;
      readonly virtualCommitId?: GitObjectId;
    }
  | {
      readonly kind: 'branch_commit';
      readonly branchRef: BranchRef;
      readonly objectId: GitObjectId;
    };

export type BuildWorkspacePlan =
  | { readonly kind: 'borrow_physical_worktree'; readonly worktreeId: WorktreeId }
  | { readonly kind: 'borrow_selected_worktree'; readonly associationId: WorktreeAssociationId }
  | { readonly kind: 'create_managed_branch_worktree'; readonly branchRef: BranchRef }
  | {
      readonly kind: 'create_owned_build_worktree';
      readonly originatingAssociationId?: WorktreeAssociationId;
    };

export interface CreateBuildRequest {
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef | null;
  readonly name: string;
  readonly source: CreateBuildSource;
  readonly workspacePlan: BuildWorkspacePlan;
}

export interface BuildAttemptFailure {
  readonly category:
    'source_changed' | 'provisioning' | 'toolchain' | 'build' | 'output' | 'interrupted';
  readonly stage: string;
  readonly summary: string;
}

export interface ReviewOperationAttempt {
  readonly attemptId: BuildAttemptId;
  readonly executionState: 'pending' | 'running' | 'completed' | 'interrupted';
  readonly outcome: 'not_completed' | 'succeeded' | 'failed' | 'unknown';
  readonly stage: string;
  readonly startedAt: string;
  readonly completedAt?: string;
  readonly failure?: BuildAttemptFailure;
}

export type BuildOutputState =
  | { readonly state: 'not_produced' }
  | {
      readonly state: 'unavailable';
      readonly summary: string;
      readonly buildOutputId?: BuildOutputId;
    }
  | {
      readonly state: 'available';
      readonly buildOutputId: BuildOutputId;
      readonly storageLabel: string;
    }
  | {
      readonly state: 'removed';
      readonly buildOutputId: BuildOutputId;
      readonly removedAt: string;
    };

export type CleanupState =
  | { readonly state: 'retained'; readonly policy: string }
  | { readonly state: 'not_eligible'; readonly reason: string }
  | { readonly state: 'eligible'; readonly reason: string }
  | {
      readonly state: 'running';
      readonly cleanupJobId: CleanupJobId;
      readonly completedEffects: number;
      readonly totalEffects: number;
    }
  | { readonly state: 'failed'; readonly cleanupJobId: CleanupJobId; readonly summary: string }
  | {
      readonly state: 'complete';
      readonly cleanupReceiptId: CleanupReceiptId;
      readonly completedAt: string;
      readonly summary: string;
    };

export interface ReviewBuild {
  readonly buildId: BuildId;
  readonly name: string;
  readonly branchRef: BranchRef | null;
  readonly source: ReviewBuildSource;
  readonly workspace: {
    readonly worktreeId: WorktreeId;
    readonly ownership: 'borrowed_external' | 'managed_branch_worktree' | 'owned_build_worktree';
    readonly locationLabel: string;
    readonly lifecycle: 'ready' | 'missing' | 'removal_pending' | 'removed' | 'unverified';
  };
  readonly latestAttempt?: ReviewOperationAttempt;
  readonly output: BuildOutputState;
  readonly cleanup: CleanupState;
  readonly attention?: {
    readonly category: 'cleanup_coordination';
    readonly summary: string;
    readonly recordedAt: string;
  };
}

export interface WorktreeReviewOverview {
  readonly repositories: readonly ReviewRepository[];
  readonly selectedRepositoryId?: RepositoryId;
  readonly branches: readonly ReviewBranch[];
  readonly activeBuildContext?: {
    readonly buildId: BuildId;
    readonly worktreeId: WorktreeId;
  };
}

export interface AssociateWorktreeRequest {
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef;
  readonly worktreeId: WorktreeId;
  readonly baseline:
    | { readonly kind: 'observed_current_head' }
    | { readonly kind: 'selected_commit'; readonly objectId: GitObjectId };
}

export interface CreateWorktreeRequest {
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef;
}

export interface OpenBuildRequest {
  readonly buildId: BuildId;
}
