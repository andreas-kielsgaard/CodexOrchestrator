export interface HumanReviewSource {
  readonly sourceRef: string;
  readonly label: string;
  readonly revision: string;
  readonly compatibility: 'compatible' | 'incompatible';
  readonly compatibilityMessage: string;
}

export interface HumanReviewInstance {
  readonly instanceRef: string;
  readonly name: string;
  readonly sourceLabel: string;
  readonly phase: string;
  readonly health: string;
  readonly stale: boolean;
  readonly build: 'not-built' | 'passed' | 'failed' | 'superseded' | 'rebuild-required';
  readonly canFocus: boolean;
  readonly purpose: string;
  readonly currentUse: string;
  readonly retention: string;
  readonly cleanup: string;
  readonly actionRequired: boolean;
  readonly actionSummary: string;
  readonly compatibility: 'compatible' | 'incompatible';
}

export interface HumanReviewOperationProgress {
  readonly operationRef: string;
  readonly operation: 'prepare' | 'build' | 'start';
  readonly state: 'pending' | 'succeeded' | 'failed';
  readonly stage: string;
  readonly stageLabel: string;
  readonly activity: 'working' | 'quiet' | 'finished';
  readonly elapsedMs: number;
  readonly evidenceAgeMs: number;
  readonly recentOutput: readonly string[];
  readonly condition: string;
  readonly expectedWait: string;
  readonly actionRequired: boolean;
  readonly actionGuidance: string;
  readonly reusableSummary: string;
  readonly missingReadinessFact?: string;
}

export interface HumanReviewOperationHistory {
  readonly operationRef: string;
  readonly operation: 'prepare' | 'build' | 'start';
  readonly state: 'pending' | 'succeeded' | 'failed';
  readonly stageLabel: string;
  readonly startedAtMs: number;
  readonly updatedAtMs: number;
  readonly output: readonly string[];
  readonly outputComplete: boolean;
}

export interface HumanReviewArtifact {
  readonly label: string;
  readonly state: 'available' | 'not-produced' | 'retained';
  readonly summary: string;
}

export interface HumanReviewLifecycleEvent {
  readonly occurredAtMs: number;
  readonly kind: string;
  readonly summary: string;
}

export interface HumanReviewRetention {
  readonly policy: string;
  readonly cleanup: string;
  readonly automatic: boolean;
  readonly actionRequired: boolean;
}

export interface HumanReviewLauncherClient {
  listSources(): Promise<readonly HumanReviewSource[]>;
  listInstances(): Promise<readonly HumanReviewInstance[]>;
  prepare(operationRef: string, sourceRef: string, name: string): Promise<HumanReviewInstance>;
  build(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  start(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  progress(operationRef: string): Promise<HumanReviewOperationProgress>;
  listProgress(): Promise<readonly HumanReviewOperationProgress[]>;
  detail(instanceRef: string): Promise<import('./worktreeBuild').WorktreeBuildDetail>;
  comparison(instanceRef: string): import('./fileReview').FileReviewSource;
  proofNavigation?(): Promise<'worktree-review' | null>;
  proofDetailNavigation?(): Promise<HumanReviewDetailNavigation | null>;
  status(instanceRef: string): Promise<HumanReviewInstance>;
  focus(instanceRef: string): Promise<HumanReviewInstance>;
  stop(instanceRef: string): Promise<HumanReviewInstance>;
  recover(instanceRef: string): Promise<HumanReviewInstance>;
}

export interface HumanReviewDetailNavigation {
  readonly instanceRef: string;
  readonly sequence: string;
}
