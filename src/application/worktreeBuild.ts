import type { FileReviewSource } from './fileReview';
import type {
  HumanReviewArtifact,
  HumanReviewLifecycleEvent,
  HumanReviewOperationHistory,
  HumanReviewRetention,
} from './humanReviewLauncher';

export interface WorktreeCommit {
  readonly id: string;
  readonly abbreviatedId: string;
  readonly message: string;
  readonly committedAt: string;
}

export interface WorktreeDirtyState {
  readonly dirty: boolean;
  readonly staged: number;
  readonly unstaged: number;
  readonly untracked: number;
}

export interface WorktreeBuildContext {
  readonly name: string;
  readonly branch?: string;
  readonly detached: boolean;
  readonly head: WorktreeCommit;
  readonly dirty: WorktreeDirtyState;
  readonly main: {
    readonly branch?: string;
    readonly detached: boolean;
    readonly head: WorktreeCommit;
    readonly dirty: WorktreeDirtyState;
  };
  readonly relationship: {
    readonly ahead: number;
    readonly behind: number;
    readonly mergeBase?: string;
    readonly summary: string;
  };
  readonly relatedBranches: readonly {
    readonly name: string;
    readonly ahead: number;
    readonly behind: number;
    readonly mergeBase?: string;
    readonly summary: string;
  }[];
  readonly history: readonly WorktreeCommit[];
  readonly comparisonBasis: string;
}

export interface WorktreeBuildDetail {
  readonly instanceRef: string;
  readonly name: string;
  readonly sourceLabel: string;
  readonly purpose: string;
  readonly phase: string;
  readonly health: string;
  readonly stale: boolean;
  readonly build: 'not-built' | 'passed' | 'failed' | 'superseded' | 'rebuild-required';
  readonly compatibility: 'compatible' | 'incompatible';
  readonly compatibilityMessage: string;
  readonly orientation: string;
  readonly prepareProduced: string;
  readonly buildProduced: string;
  readonly openProduced: string;
  readonly currentCondition: string;
  readonly actionRequired: boolean;
  readonly actionSummary: string;
  readonly reusableSummary: string;
  readonly retention: HumanReviewRetention;
  readonly artifacts: readonly HumanReviewArtifact[];
  readonly lifecycleHistory: readonly HumanReviewLifecycleEvent[];
  readonly operations: readonly HumanReviewOperationHistory[];
  readonly context: WorktreeBuildContext;
}

export interface WorktreeBuildClient {
  context(): Promise<WorktreeBuildContext>;
  detail(): Promise<WorktreeBuildDetail>;
  comparison: FileReviewSource;
  markReady(): Promise<void>;
  proofNavigation(): Promise<WorktreeProofNavigation | null>;
}

export interface WorktreeProofNavigation {
  readonly route: 'application' | 'worktree-details' | 'file-review';
  readonly sequence: string;
}
