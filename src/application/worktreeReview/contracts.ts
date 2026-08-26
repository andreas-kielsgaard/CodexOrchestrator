export type OpaqueId<Kind extends string> = string & { readonly __kind?: Kind };

export type RepositoryId = OpaqueId<'repository'>;
export type BranchRef = OpaqueId<'branch-ref'>;
export type WorktreeId = OpaqueId<'worktree'>;
export type WorktreeAssociationId = OpaqueId<'worktree-association'>;
export type BuildId = OpaqueId<'review-build'>;
export type BuildAttemptId = OpaqueId<'build-attempt'>;
export type ArtifactSetId = OpaqueId<'artifact-set'>;
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
  readonly artifactStorage: CapabilityAvailability;
}

export interface ReviewRepository {
  readonly repositoryId: RepositoryId;
  readonly name: string;
  /** Presentation-only. Never use this path as repository identity. */
  readonly locationLabel: string;
  readonly readiness: RepositoryReadiness;
}

export interface ReviewBranch {
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef;
  readonly displayName: string;
  readonly tip: GitCommit;
  readonly aheadOfDefault: number;
  readonly behindDefault: number;
  readonly associatedWorktreeCount: number;
}

export interface ApplicationMetadataItem {
  readonly label: string;
  readonly value: string;
  readonly source: 'committed-branch-tip' | 'worktree';
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
  readonly associationId: WorktreeAssociationId;
  readonly worktreeId: WorktreeId;
  readonly branchRef: BranchRef;
  readonly name: string;
  /** Presentation-only. Durable operations use worktreeId and associationId. */
  readonly locationLabel: string;
  readonly provenance:
    | 'git_branch_checkout'
    | 'user_associated_detached_checkout'
    | 'worktree_review_created';
  readonly ownership: 'borrowed_external' | 'managed_branch_worktree' | 'owned_build_worktree';
  readonly baseline: AssociationBaseline;
  readonly currentHead: GitCommit;
  readonly observedStateFingerprint: string;
  readonly changes: WorktreeChanges;
  readonly detachedHead: boolean;
  readonly branchReachability: 'reachable' | 'not_reachable' | 'unknown';
  readonly availability: WorktreeAvailability;
  readonly applicationMetadata: readonly ApplicationMetadataItem[];
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
  readonly applicationMetadata: readonly ApplicationMetadataItem[];
  readonly worktrees: readonly AssociatedWorktree[];
  readonly associationCandidates: readonly WorktreeAssociationCandidate[];
  readonly builds: readonly ReviewBuild[];
}

export interface BranchHistoryPage {
  readonly commits: readonly GitCommit[];
  readonly nextCursor?: string;
}

export type ReviewBuildSource =
  | {
      readonly kind: 'existing_worktree';
      readonly associationId: WorktreeAssociationId;
      readonly expectedHead: GitObjectId;
      readonly stateFingerprint: string;
    }
  | {
      readonly kind: 'worktree_snapshot';
      readonly associationId: WorktreeAssociationId;
      readonly baseObjectId: GitObjectId;
      readonly stateFingerprint: string;
    }
  | {
      readonly kind: 'branch_commit';
      readonly branchRef: BranchRef;
      readonly objectId: GitObjectId;
    };

export type BuildWorkspacePlan =
  | { readonly kind: 'borrow_selected_worktree'; readonly associationId: WorktreeAssociationId }
  | { readonly kind: 'create_managed_branch_worktree'; readonly branchRef: BranchRef }
  | {
      readonly kind: 'create_owned_build_worktree';
      readonly originatingAssociationId?: WorktreeAssociationId;
      readonly objectId: GitObjectId;
    };

export interface CreateBuildRequest {
  readonly repositoryId: RepositoryId;
  readonly branchRef: BranchRef;
  readonly name: string;
  readonly source: ReviewBuildSource;
  readonly workspacePlan: BuildWorkspacePlan;
}

export interface BuildAttemptFailure {
  readonly category:
    'source_changed' | 'provisioning' | 'toolchain' | 'build' | 'artifact' | 'interrupted';
  readonly stage: string;
  readonly summary: string;
}

export interface ReviewOperationAttempt {
  readonly attemptId: BuildAttemptId;
  readonly executionState: 'pending' | 'running' | 'completed' | 'interrupted';
  readonly verdict: 'not_evaluated' | 'passed' | 'failed' | 'unknown';
  readonly stage: string;
  readonly startedAt: string;
  readonly completedAt?: string;
  readonly failure?: BuildAttemptFailure;
}

export type ArtifactState =
  | { readonly state: 'not_produced' }
  | { readonly state: 'promotion_failed'; readonly summary: string }
  | {
      readonly state: 'available';
      readonly artifactSetId: ArtifactSetId;
      readonly fileCount: number;
      readonly manifestHash: string;
      readonly storageLabel: string;
    }
  | {
      readonly state: 'removed';
      readonly artifactSetId: ArtifactSetId;
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
  readonly branchRef: BranchRef;
  readonly source: ReviewBuildSource;
  readonly workspace: {
    readonly worktreeId: WorktreeId;
    readonly ownership: 'borrowed_external' | 'managed_branch_worktree' | 'owned_build_worktree';
    readonly locationLabel: string;
    readonly lifecycle: 'ready' | 'missing' | 'removal_pending' | 'removed' | 'unverified';
  };
  readonly latestAttempt?: ReviewOperationAttempt;
  readonly artifact: ArtifactState;
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
