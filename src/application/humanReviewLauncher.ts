export interface HumanReviewSource {
  readonly sourceRef: string;
  readonly label: string;
  readonly revision: string;
}

export interface HumanReviewInstance {
  readonly instanceRef: string;
  readonly name: string;
  readonly sourceLabel: string;
  readonly phase: string;
  readonly health: string;
  readonly stale: boolean;
  readonly build: 'not-built' | 'passed' | 'failed';
  readonly canFocus: boolean;
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
}

export interface HumanReviewLauncherClient {
  listSources(): Promise<readonly HumanReviewSource[]>;
  listInstances(): Promise<readonly HumanReviewInstance[]>;
  prepare(operationRef: string, sourceRef: string, name: string): Promise<HumanReviewInstance>;
  build(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  start(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  progress(operationRef: string): Promise<HumanReviewOperationProgress>;
  listProgress(): Promise<readonly HumanReviewOperationProgress[]>;
  proofNavigation?(): Promise<'worktree-review' | null>;
  status(instanceRef: string): Promise<HumanReviewInstance>;
  focus(instanceRef: string): Promise<HumanReviewInstance>;
  stop(instanceRef: string): Promise<HumanReviewInstance>;
  recover(instanceRef: string): Promise<HumanReviewInstance>;
}
