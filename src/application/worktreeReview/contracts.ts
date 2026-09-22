export type { RepositoryId } from '../repositoryCatalog/contracts';
import type {
  OpaqueId,
  BranchRef,
  WorktreeId,
  GitObjectId,
  GitCommit,
  ReviewBranch,
  CommitSourceContext,
} from '../branches';
export type {
  OpaqueId,
  BranchRef,
  WorktreeId,
  GitObjectId,
  GitCommit,
  ReviewBranch,
  ReviewTarget,
  ActivityEstimate,
  WorktreeActivity,
  CommitSourceContext,
  HistoryScope,
  CommitHistoryQuery,
  CommitHistoryPage,
  GraphAnchor,
  GraphConnection,
  BranchGraphData,
} from '../branches';
export type CleanupReceiptId = OpaqueId<'cleanup-receipt'>;
export type CleanupJobId = OpaqueId<'cleanup-job'>;
export type BuildOutputId = OpaqueId<'build-output'>;
export type BuildAttemptId = OpaqueId<'build-attempt'>;
export type BuildId = OpaqueId<'review-build'>;
export type WorktreeAssociationId = OpaqueId<'worktree-association'>;
import type { RepositoryId } from '../repositoryCatalog/contracts';

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

export interface DetachedWorktreeDetail {
  readonly worktreeId: WorktreeId;
  readonly locationLabel: string;
  readonly head: GitCommit;
  readonly stagedFiles: number;
  readonly unstagedFiles: number;
  readonly untrackedFiles: number;
  readonly lastActivity: import('../branches').ActivityEstimate | null;
  readonly recordedBranches: readonly string[];
  /** Orchid-recorded association or owned-workspace time, not a guessed Git creation date. */
  readonly firstRecordedAt: string | null;
  readonly diskSizeBytes: number | null;
  readonly diskSizeLimited: boolean;
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

export type ApplicationBuildProfile = 'release' | 'debug';

export interface CreateBuildRequest {
  readonly profile: ApplicationBuildProfile;
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

export interface BuildLogChunk {
  readonly text: string;
  readonly nextOffset: number;
  readonly truncatedBefore: boolean;
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
  readonly profile?: ApplicationBuildProfile | null;
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
