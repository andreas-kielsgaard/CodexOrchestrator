import type { AgentReviewApplicationMode } from './contracts';

export type AgentReviewInstanceMode = Exclude<AgentReviewApplicationMode, 'production'>;

export type AgentReviewInstanceCapability =
  | 'renderer-endpoint'
  | 'owned-native-window'
  | 'inspectable-webview'
  | 'native-ipc'
  | 'frontend-logs'
  | 'backend-logs';

export interface AgentReviewWorktreeExpectation {
  readonly worktreePath: string;
  readonly gitCommit: string;
  readonly sourceFingerprint: string;
}

export interface AgentReviewInstanceRequest {
  readonly kind: 'build-and-launch-worktree-review-instance';
  readonly id: string;
  readonly identity: AgentReviewWorktreeExpectation;
  readonly applicationMode: AgentReviewInstanceMode;
  readonly requiredCapabilities: readonly AgentReviewInstanceCapability[];
  readonly isolation: Readonly<{
    applicationData: 'isolated';
    credentials: 'scrubbed';
    ports: 'ephemeral';
  }>;
  readonly cleanup: 'required';
}

export interface AgentReviewWorktreeIdentity extends AgentReviewWorktreeExpectation {
  readonly instanceId: string;
  readonly sessionId: string;
  readonly buildId: string;
  readonly tauriIdentifier: string;
}

export interface AgentReviewOwnedEndpoint {
  readonly kind: 'renderer-http';
  readonly origin: string;
  readonly owner: 'worktree-runtime';
}

export interface AgentReviewOwnedWindow {
  readonly kind: 'native-window';
  readonly windowRef: string;
  readonly label: string;
  readonly owner: 'worktree-runtime';
}

export interface AgentReviewInstanceEvidenceRoots {
  readonly runtimeRoot: string;
  readonly runtimeManifestPath: string;
  readonly reviewRoot: string;
}

export interface AgentReviewObservedIsolation {
  readonly applicationData: Readonly<{
    kind: 'isolated';
    root: string;
  }>;
  readonly credentials: 'scrubbed';
  readonly ports: 'ephemeral';
}

export interface AgentReviewRunningInstance {
  readonly kind: 'worktree-review-instance';
  readonly requestId: string;
  readonly identity: AgentReviewWorktreeIdentity;
  readonly applicationMode: AgentReviewInstanceMode;
  readonly lifecycle: Readonly<{
    state: 'running';
    owner: 'worktree-runtime';
    cleanup: 'required';
  }>;
  readonly isolation: AgentReviewObservedIsolation;
  readonly capabilities: readonly AgentReviewInstanceCapability[];
  readonly rendererEndpoint: AgentReviewOwnedEndpoint | null;
  readonly nativeWindow: AgentReviewOwnedWindow | null;
  readonly evidence: AgentReviewInstanceEvidenceRoots;
  readonly observedAt: string;
}

export interface AgentReviewStoppedInstance {
  readonly kind: 'worktree-review-instance-stopped';
  readonly instanceId: string;
  readonly lifecycle: 'stopped';
  readonly endpointsReleased: boolean;
  readonly windowsReleased: boolean;
  readonly isolatedStateRemoved: boolean;
  readonly runtimeManifestFinalized: boolean;
}

/** Application-facing lifecycle port. Driver protocols belong to review adapters. */
export interface AgentReviewWorktreeRuntime {
  buildAndLaunch(request: AgentReviewInstanceRequest): Promise<AgentReviewRunningInstance>;
  readInstance(
    instanceId: string,
  ): Promise<AgentReviewRunningInstance | AgentReviewStoppedInstance>;
  stopInstance(instanceId: string, reason: string): Promise<AgentReviewStoppedInstance>;
}

export interface AgentReviewInstanceReadiness {
  readonly ready: boolean;
  readonly reasons: readonly string[];
}

export function evaluateAgentReviewInstance(
  request: AgentReviewInstanceRequest,
  instance: AgentReviewRunningInstance,
): AgentReviewInstanceReadiness {
  const reasons: string[] = [];
  const expected = request.identity;
  const observed = instance.identity;

  if (instance.requestId !== request.id) reasons.push('request identity does not match');
  if (observed.worktreePath !== expected.worktreePath) reasons.push('worktree path does not match');
  if (observed.gitCommit !== expected.gitCommit) reasons.push('git commit does not match');
  if (observed.sourceFingerprint !== expected.sourceFingerprint) {
    reasons.push('source fingerprint does not match');
  }
  if (instance.applicationMode !== request.applicationMode) {
    reasons.push('application mode does not match');
  }
  if (instance.lifecycle.owner !== 'worktree-runtime')
    reasons.push('instance is not runtime-owned');
  if (instance.lifecycle.cleanup !== 'required') reasons.push('cleanup is not required');
  if (!instance.isolation.applicationData.root)
    reasons.push('isolated application data is missing');
  if (instance.isolation.credentials !== request.isolation.credentials) {
    reasons.push('credential isolation does not match');
  }
  if (instance.isolation.ports !== request.isolation.ports) {
    reasons.push('port isolation does not match');
  }

  const capabilities = new Set(instance.capabilities);
  for (const capability of request.requiredCapabilities) {
    if (!capabilities.has(capability)) reasons.push(`missing capability: ${capability}`);
  }
  if (capabilities.has('renderer-endpoint') && !instance.rendererEndpoint) {
    reasons.push('renderer endpoint is missing');
  }
  if (capabilities.has('owned-native-window') && !instance.nativeWindow) {
    reasons.push('native window is missing');
  }
  if (!instance.evidence.runtimeRoot || !instance.evidence.runtimeManifestPath) {
    reasons.push('runtime evidence location is missing');
  }
  if (!instance.evidence.reviewRoot) reasons.push('review evidence root is missing');

  return { ready: reasons.length === 0, reasons };
}
