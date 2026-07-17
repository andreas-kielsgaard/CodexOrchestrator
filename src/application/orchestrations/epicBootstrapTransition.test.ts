import { describe, expect, it } from 'vitest';
import {
  decodeEpicBootstrapTransitionQueryV2,
  projectBootstrapTransitionStatus,
} from './epicBootstrapTransition';

const attempt = {
  attemptId: 'attempt-0',
  ordinal: 0,
  agentSessionId: 'bootstrap-session',
  agentInvocationId: 'bootstrap-invocation',
  launchedAt: null,
  lifecycleStatus: null,
  lifecycleObservedAt: null,
  semanticCompletionFactId: null,
  semanticCompletedAt: null,
  retryDisposition: 'active',
  retryReason: null,
  retryAttemptId: null,
  acceptedAt: null,
};
function query(overrides: Record<string, unknown> = {}) {
  return {
    contract: 'epic-bootstrap-transition-query/v2',
    schemaVersion: 2,
    transitions: [
      {
        initiationId: 'initiation-1',
        epicId: 'epic-1',
        preparationId: 'preparation-1',
        preparedRoot: 'root',
        approvedPlanPath: 'plan',
        manifestPath: 'manifest',
        overviewPath: 'overview',
        runnerBriefPath: 'brief',
        bootstrapSessionId: 'bootstrap-session',
        bootstrapInvocationId: 'bootstrap-invocation',
        preparedAt: null,
        bootstrapSessionCreatedAt: null,
        bootstrapLaunchedAt: null,
        bootstrapLifecycleStatus: null,
        bootstrapLifecycleObservedAt: null,
        semanticCompletionFactId: null,
        semanticCompletedAt: null,
        materialAcceptedAt: null,
        runnerSessionId: 'runner-session',
        runnerInvocationId: 'runner-invocation',
        runnerSessionCreatedAt: null,
        runnerLaunchedAt: null,
        runnerLifecycleStatus: null,
        runnerLifecycleObservedAt: null,
        currentAttemptId: 'attempt-0',
        retryState: 'active',
        blockedReason: null,
        acceptedAttemptId: null,
        bootstrapAttempts: [attempt],
        ...overrides,
      },
    ],
  };
}

describe('Epic Bootstrap transition v2', () => {
  it('strictly decodes and rejects unknown state or broken attempt correlation', () => {
    expect(decodeEpicBootstrapTransitionQueryV2(query()).transitions).toHaveLength(1);
    expect(() => decodeEpicBootstrapTransitionQueryV2({ ...query(), extra: true })).toThrow(
      'unknown field',
    );
    expect(() => decodeEpicBootstrapTransitionQueryV2(query({ retryState: 'guessing' }))).toThrow(
      'unknown retry state',
    );
    expect(() =>
      decodeEpicBootstrapTransitionQueryV2(query({ currentAttemptId: 'other' })),
    ).toThrow('final ordered attempt');
  });

  it.each([
    [{}, 'preparing'],
    [{ preparedAt: 't' }, 'preparing'],
    [{ preparedAt: 't', bootstrapSessionCreatedAt: 't' }, 'bootstrap_running'],
    [
      { preparedAt: 't', bootstrapSessionCreatedAt: 't', semanticCompletionFactId: 'fact' },
      'awaiting_matching_lifecycle',
    ],
    [{ preparedAt: 't', bootstrapSessionCreatedAt: 't', retryState: 'retryable' }, 'retrying'],
    [
      {
        preparedAt: 't',
        bootstrapSessionCreatedAt: 't',
        retryState: 'blocked',
        blockedReason: 'attempt ceiling reached',
      },
      'blocked',
    ],
    [
      {
        preparedAt: 't',
        bootstrapSessionCreatedAt: 't',
        retryState: 'accepted',
        materialAcceptedAt: 't',
      },
      'material_accepted',
    ],
    [
      {
        preparedAt: 't',
        bootstrapSessionCreatedAt: 't',
        retryState: 'accepted',
        materialAcceptedAt: 't',
        runnerSessionCreatedAt: 't',
      },
      'runner_launching',
    ],
    [
      {
        preparedAt: 't',
        bootstrapSessionCreatedAt: 't',
        retryState: 'accepted',
        materialAcceptedAt: 't',
        runnerSessionCreatedAt: 't',
        runnerLaunchedAt: 't',
      },
      'runner_launched',
    ],
  ])('maps durable facts truthfully for %s', (overrides, kind) => {
    const transition = decodeEpicBootstrapTransitionQueryV2(query(overrides)).transitions[0]!;
    expect(projectBootstrapTransitionStatus(transition).kind).toBe(kind);
  });
});
