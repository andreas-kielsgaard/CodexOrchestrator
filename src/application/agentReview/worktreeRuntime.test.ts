import {
  evaluateAgentReviewInstance,
  type AgentReviewInstanceRequest,
  type AgentReviewRunningInstance,
} from './worktreeRuntime';

const request = {
  kind: 'build-and-launch-worktree-review-instance',
  id: 'review-launch-1',
  identity: {
    worktreePath: 'C:/worktrees/review',
    gitCommit: '0123456789abcdef',
    sourceFingerprint: 'sha256:source',
  },
  applicationMode: 'test',
  requiredCapabilities: ['renderer-endpoint', 'native-ipc'],
  isolation: {
    applicationData: 'isolated',
    credentials: 'scrubbed',
    ports: 'ephemeral',
  },
  cleanup: 'required',
} as const satisfies AgentReviewInstanceRequest;

const instance = {
  kind: 'worktree-review-instance',
  requestId: request.id,
  identity: {
    ...request.identity,
    instanceId: 'instance-1',
    sessionId: 'session-1',
    buildId: 'build-1',
    tauriIdentifier: 'com.codex-orchestrator.review.instance-1',
  },
  applicationMode: request.applicationMode,
  lifecycle: { state: 'running', owner: 'worktree-runtime', cleanup: 'required' },
  isolation: {
    applicationData: { kind: 'isolated', root: '.dev/worktree-runtime/instance-1/app-data' },
    credentials: 'scrubbed',
    ports: 'ephemeral',
  },
  capabilities: ['renderer-endpoint', 'native-ipc'],
  rendererEndpoint: {
    kind: 'renderer-http',
    origin: 'http://127.0.0.1:41231',
    owner: 'worktree-runtime',
  },
  nativeWindow: null,
  evidence: {
    runtimeRoot: '.dev/worktree-runtime/instance-1',
    runtimeManifestPath: '.dev/worktree-runtime/instance-1/manifest.json',
    reviewRoot: '.dev/agent-review/instance-1',
  },
  observedAt: '2026-07-27T10:00:00Z',
} as const satisfies AgentReviewRunningInstance;

describe('agent review worktree-runtime handoff', () => {
  it('accepts an exact, runtime-owned instance without exposing driver commands', () => {
    expect(evaluateAgentReviewInstance(request, instance)).toEqual({ ready: true, reasons: [] });
    expect(instance).not.toHaveProperty('driver');
    expect(instance).not.toHaveProperty('command');
  });

  it('rejects stale source identity, missing capabilities, and incomplete evidence', () => {
    const stale = {
      ...instance,
      identity: { ...instance.identity, sourceFingerprint: 'sha256:stale' },
      capabilities: ['renderer-endpoint'],
      isolation: { ...instance.isolation, applicationData: { kind: 'isolated', root: '' } },
      evidence: { ...instance.evidence, runtimeManifestPath: '' },
    } satisfies AgentReviewRunningInstance;

    expect(evaluateAgentReviewInstance(request, stale)).toEqual({
      ready: false,
      reasons: [
        'source fingerprint does not match',
        'isolated application data is missing',
        'missing capability: native-ipc',
        'runtime evidence location is missing',
      ],
    });
  });
});
