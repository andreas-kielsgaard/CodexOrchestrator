export interface HumanReviewSource {
  readonly sourceRef: string;
  readonly label: string;
  readonly branch?: string;
  readonly detached: boolean;
  readonly isMain: boolean;
  readonly isCurrent: boolean;
  readonly parentSourceRef?: string;
  readonly lineageAmbiguous: boolean;
  readonly relationship: 'related' | 'unrelated';
  readonly ahead: number;
  readonly behind: number;
  readonly forkRevision: string;
  readonly revision: string;
  readonly compatibility: 'compatible' | 'incompatible' | 'unavailable';
  readonly compatibilityMessage: string;
  readonly detailsState: 'pending' | 'cached' | 'ready' | 'failed';
  readonly attached: boolean;
  readonly refKind: 'local_branch' | 'remote_branch' | 'tag' | 'archive' | 'detached';
  readonly mergedDirectly: boolean;
  readonly equivalentPatches: number;
  readonly comparisonBranch: string;
}

export interface HumanReviewSourceListOptions {
  readonly includeDetached?: boolean;
  readonly refresh?: boolean;
}

export interface HumanReviewSettings {
  readonly cleanupDetachedBuilds: boolean;
}

export interface HumanReviewSourceHistory {
  readonly branch: string;
  readonly sourceLabel: string;
  readonly revision: string;
  readonly forkRevision: string;
  readonly commitCount: number;
  readonly truncated: boolean;
  readonly commits: readonly HumanReviewCommit[];
  readonly lineageMarkers: readonly HumanReviewLineageMarker[];
}

export interface HumanReviewCommit {
  readonly id: string;
  readonly abbreviatedId: string;
  readonly subject: string;
  readonly description: string;
  readonly author: string;
  readonly committedAt: string;
  readonly filesChanged: number;
  readonly insertions: number;
  readonly deletions: number;
}

export interface HumanReviewLineageMarker {
  readonly branch: string;
  readonly commitId: string;
  readonly abbreviatedId: string;
}

export interface HumanReviewInstance {
  readonly instanceRef: string;
  readonly name: string;
  readonly sourceRef: string;
  readonly sourceLabel: string;
  readonly preparedRevision: string | null;
  readonly currentRevision: string | null;
  readonly sourceState: 'current' | 'outdated' | 'changed' | 'unavailable' | 'unknown';
  readonly outdatedByCommits: number | null;
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
  listSources(options?: HumanReviewSourceListOptions): Promise<readonly HumanReviewSource[]>;
  listRepositoryHistory(): Promise<readonly HumanReviewSource[]>;
  attachWorktree(sourceRef: string): Promise<HumanReviewSource>;
  sourceHistory(sourceRef: string): Promise<HumanReviewSourceHistory>;
  listInstances(): Promise<readonly HumanReviewInstance[]>;
  settings(): Promise<HumanReviewSettings>;
  updateSettings(settings: HumanReviewSettings): Promise<HumanReviewSettings>;
  prepare(operationRef: string, sourceRef: string, name: string): Promise<HumanReviewInstance>;
  build(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  start(operationRef: string, instanceRef: string): Promise<HumanReviewInstance>;
  progress(operationRef: string): Promise<HumanReviewOperationProgress>;
  listProgress(): Promise<readonly HumanReviewOperationProgress[]>;
  detail(instanceRef: string): Promise<import('./worktreeBuild').WorktreeBuildDetail>;
  comparison(instanceRef: string): import('./fileReview').FileReviewSource;
  status(instanceRef: string): Promise<HumanReviewInstance>;
  focus(instanceRef: string): Promise<HumanReviewInstance>;
  stop(instanceRef: string): Promise<HumanReviewInstance>;
  recover(instanceRef: string): Promise<HumanReviewInstance>;
}
