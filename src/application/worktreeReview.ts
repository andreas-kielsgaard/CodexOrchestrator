import type {
  HumanReviewInstance,
  HumanReviewLauncherClient,
  HumanReviewOperationProgress,
  HumanReviewSettings,
  HumanReviewSource,
  HumanReviewSourceHistory,
} from './humanReviewLauncher';

export type WorktreeReviewSource = HumanReviewSource;
export type WorktreeReviewSourceHistory = HumanReviewSourceHistory;
export type WorktreeReviewInstance = HumanReviewInstance;
export type WorktreeReviewOperationProgress = HumanReviewOperationProgress;
export type WorktreeReviewSettings = HumanReviewSettings;

export type WorktreeReviewReadinessStatus =
  | 'ready'
  | 'needsRepository'
  | 'missingTool'
  | 'runtimeUnavailable'
  | 'storageUnavailable'
  | 'repositoryUnavailable';

export interface WorktreeReviewReadiness {
  readonly status: WorktreeReviewReadinessStatus;
  readonly message: string;
  readonly repositoryRoot?: string;
}

export interface WorktreeReviewCommandError {
  readonly code: string;
  readonly message: string;
}

export function worktreeReviewErrorMessage(cause: unknown, fallback: string): string {
  if (cause instanceof Error) return cause.message;
  if (
    typeof cause === 'object' &&
    cause !== null &&
    'message' in cause &&
    typeof cause.message === 'string'
  ) {
    return cause.message;
  }
  return typeof cause === 'string' && cause ? cause : fallback;
}

/** Product boundary. Development proof commands are intentionally not part of this contract. */
export interface WorktreeReviewClient extends HumanReviewLauncherClient {
  readiness(): Promise<WorktreeReviewReadiness>;
  selectRepository(repositoryRoot: string): Promise<WorktreeReviewReadiness>;
}
