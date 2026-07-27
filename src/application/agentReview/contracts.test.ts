import {
  agentReviewDispositions,
  type AgentReviewEvidenceBundle,
  type AgentReviewRequest,
  type AgentReviewResult,
} from './contracts';

const request = {
  id: 'review-request-1',
  revision: '0123456789abcdef',
  worktree: 'C:/worktrees/review',
  surface: { id: 'agent-session', name: 'Agent Session' },
  scenario: {
    id: 'send-message',
    name: 'Send a message',
    startingState: 'An idle recorded Agent Session is open.',
    actions: ['Enter a message', 'Submit the message'],
  },
  environment: {
    platform: 'Windows 11',
    viewport: { width: 1440, height: 900 },
  },
  claims: [
    {
      id: 'message-visible',
      kind: 'behavior',
      statement: 'The submitted message appears in the session.',
    },
    {
      id: 'layout-visible',
      kind: 'visual',
      statement: 'The message remains visible at the requested viewport.',
    },
  ],
  capabilities: [
    { id: 'inspect-surface', purpose: 'Inspect the rendered surface.' },
    { id: 'interact-with-surface', purpose: 'Exercise the named scenario.' },
  ],
  evidenceRequirements: [
    { claimId: 'message-visible', evidenceKinds: ['action-log', 'assertion'] },
    { claimId: 'layout-visible', evidenceKinds: ['screenshot', 'assertion'] },
  ],
} as const satisfies AgentReviewRequest;

describe('agent application review contracts', () => {
  it('represents deterministic renderer evidence without driver commands', () => {
    const bundle = {
      id: 'renderer-bundle',
      requestId: request.id,
      lane: 'deterministic-verification',
      applicationMode: 'test',
      driver: { name: 'renderer-adapter', version: 'test-version' },
      environment: request.environment,
      recordedAt: '2026-07-17T08:00:00Z',
      startingState: request.scenario.startingState,
      actions: [
        { sequence: 1, description: 'Enter a message', driverOutcome: 'completed' },
        { sequence: 2, description: 'Submit the message', driverOutcome: 'completed' },
      ],
      assertions: [
        {
          claimId: 'message-visible',
          description: 'The submitted message was observed in the session.',
          outcome: 'passed',
        },
        {
          claimId: 'layout-visible',
          description: 'The retained screenshot uses the requested viewport.',
          outcome: 'passed',
        },
      ],
      producedFiles: [
        { path: 'evidence/renderer/message.png', kind: 'screenshot' },
        { path: 'evidence/renderer/results.json', kind: 'artifact' },
      ],
      runtimeEvidence: {
        instanceId: 'review-instance-1',
        runtimeManifestPath: 'evidence/runtime/manifest.json',
      },
      observations: ['The scenario completed from its declared starting state.'],
      unverifiedClaims: [],
    } as const satisfies AgentReviewEvidenceBundle;

    expect(bundle).toMatchObject({
      lane: 'deterministic-verification',
      applicationMode: 'test',
      driver: { name: 'renderer-adapter', version: 'test-version' },
      environment: { viewport: { width: 1440, height: 900 } },
    });
    expect(bundle.producedFiles).toHaveLength(2);
  });

  it('represents native exploration without pretending that it is deterministic proof', () => {
    const bundle = {
      id: 'native-bundle',
      requestId: request.id,
      lane: 'exploratory-control',
      applicationMode: 'development',
      driver: { name: 'native-attachment-adapter', version: null },
      environment: { platform: 'Windows 11', viewport: null },
      recordedAt: '2026-07-17T08:05:00Z',
      startingState: 'A development-only native shell is running.',
      actions: [
        {
          sequence: 1,
          description: 'Inspect the shell surface',
          driverOutcome: 'completed',
        },
      ],
      assertions: [
        {
          claimId: 'message-visible',
          description: 'Native shell behavior was not deterministically verified.',
          outcome: 'not-run',
        },
      ],
      producedFiles: [{ path: 'evidence/native/observation.json', kind: 'native-observation' }],
      runtimeEvidence: null,
      observations: ['The development shell accepted an inspection attachment.'],
      unverifiedClaims: ['message-visible'],
    } as const satisfies AgentReviewEvidenceBundle;

    expect(bundle).toMatchObject({
      lane: 'exploratory-control',
      applicationMode: 'development',
      driver: { version: null },
      environment: { viewport: null },
      unverifiedClaims: ['message-visible'],
    });
  });

  it('keeps agent judgement and each disposition distinct from acquired evidence', () => {
    const results = agentReviewDispositions.map((disposition): AgentReviewResult => ({
      kind: 'agent-judgement',
      id: `result-${disposition}`,
      requestId: request.id,
      evidenceBundleIds: ['renderer-bundle'],
      disposition,
      summary: `Review disposition: ${disposition}`,
      findings: [],
    }));

    expect(agentReviewDispositions).toEqual([
      'accepted',
      'changes-required',
      'user-review-required',
      'blocked',
      'inconclusive',
    ]);
    expect(new Set(results.map(({ disposition }) => disposition))).toHaveLength(5);
    expect(results.every(({ kind }) => kind === 'agent-judgement')).toBe(true);
  });
});
