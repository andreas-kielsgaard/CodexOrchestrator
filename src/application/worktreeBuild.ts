import type { FileReviewSource } from './fileReview';

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
  readonly history: readonly WorktreeCommit[];
  readonly comparisonBasis: string;
}

export interface WorktreeBuildClient {
  context(): Promise<WorktreeBuildContext>;
  comparison: FileReviewSource;
  markReady(): Promise<void>;
  proofNavigation(): Promise<WorktreeProofNavigation | null>;
}

export interface WorktreeProofNavigation {
  readonly route: 'application' | 'worktree-details' | 'file-review';
  readonly sequence: string;
}
