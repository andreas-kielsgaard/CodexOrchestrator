import { describe, expect, it } from 'vitest';
import {
  decodeSprintRunnerTransitionQueryV1,
  projectSprintRunnerTransitionStatus,
} from './sprintRunnerTransition';

const query = () => ({
  contract: 'sprint-runner-transition-query/v1',
  transitions: [
    {
      sprintId: 'sprint-1',
      epicId: 'epic-1',
      requestId: 'request-1',
      epicRunnerInvocationId: 'epic-runner-1',
      sprintRunnerSessionId: 'sprint-runner-session-1',
      sprintRunnerInvocationId: 'sprint-runner-1',
      requestedAt: '2026-08-02T00:00:00Z',
      authorizedAt: '2026-08-02T00:00:01Z',
      sessionCreatedAt: '2026-08-02T00:00:02Z',
      harnessAppliedAt: '2026-08-02T00:00:03Z',
      launchAcceptedAt: '2026-08-02T00:00:04Z',
      preStartReady: true,
      lifecycleObserved: false,
      accepted: false,
      downstreamNotStarted: true,
    },
  ],
});

describe('Sprint Runner transition query', () => {
  it('strictly decodes launch-accepted pre-start state without inventing observation or acceptance', () => {
    const transition = decodeSprintRunnerTransitionQueryV1(query()).transitions[0]!;
    expect(projectSprintRunnerTransitionStatus(transition)).toMatchObject({
      label: 'Sprint Runner launch accepted — pre-start ready',
      preStartReady: true,
      lifecycleObserved: false,
      accepted: false,
    });
  });

  it('rejects unsupported fields and inconsistent evidence', () => {
    expect(() => decodeSprintRunnerTransitionQueryV1({ ...query(), extra: true })).toThrow(
      'unsupported fields',
    );
    const inconsistent = query();
    inconsistent.transitions[0]!.preStartReady = false;
    expect(() => decodeSprintRunnerTransitionQueryV1(inconsistent)).toThrow('inconsistent');
  });

  it('requires both pre-start evidence boundaries before accepting and started reevaluation before planning-ready', () => {
    const accepted = query();
    Object.assign(accepted.transitions[0]!, {
      lifecycleObserved: true,
      accepted: true,
      preStartSemanticOutcomeRecordedAt: '2026-08-02T00:00:05Z',
      preStartLifecycleObservedAt: '2026-08-02T00:00:06Z',
      preStartOutcomeAcceptedAt: '2026-08-02T00:00:07Z',
    });
    expect(decodeSprintRunnerTransitionQueryV1(accepted).transitions[0]!.accepted).toBe(true);
    const incomplete = query();
    Object.assign(incomplete.transitions[0]!, {
      sprintStartPersistedAt: '2026-08-02T00:00:08Z',
      planningReadyAt: '2026-08-02T00:00:09Z',
    });
    expect(() => decodeSprintRunnerTransitionQueryV1(incomplete)).toThrow('planning-ready');
  });

  it('projects each durable phase without calling persisted delivery or activation observed', () => {
    const phase = (fields: Record<string, unknown>) => {
      const value = query();
      Object.assign(value.transitions[0]!, fields);
      return projectSprintRunnerTransitionStatus(
        decodeSprintRunnerTransitionQueryV1(value).transitions[0]!,
      );
    };
    expect(
      phase({ preStartSemanticOutcomeRecordedAt: '2026-08-02T00:00:05Z' }).label,
    ).toBe('Pre-start outcome recorded; matching lifecycle pending');
    expect(
      phase({
        lifecycleObserved: true,
        accepted: true,
        preStartSemanticOutcomeRecordedAt: '2026-08-02T00:00:05Z',
        preStartLifecycleObservedAt: '2026-08-02T00:00:06Z',
        preStartOutcomeAcceptedAt: '2026-08-02T00:00:07Z',
      }).label,
    ).toBe('Pre-start outcome accepted; Epic continuation delivery pending');
    expect(
      phase({
        lifecycleObserved: true,
        accepted: true,
        preStartSemanticOutcomeRecordedAt: '2026-08-02T00:00:05Z',
        preStartLifecycleObservedAt: '2026-08-02T00:00:06Z',
        preStartOutcomeAcceptedAt: '2026-08-02T00:00:07Z',
        parentContinuationDeliveryRequestedAt: '2026-08-02T00:00:08Z',
        parentContinuationDeliveryPersistedAt: '2026-08-02T00:00:09Z',
      }).label,
    ).toBe('Epic continuation invocation persisted; launch acceptance pending');
    const waiting = phase({
      lifecycleObserved: true,
      accepted: true,
      preStartSemanticOutcomeRecordedAt: '2026-08-02T00:00:05Z',
      preStartLifecycleObservedAt: '2026-08-02T00:00:06Z',
      preStartOutcomeAcceptedAt: '2026-08-02T00:00:07Z',
      parentContinuationDeliveryRequestedAt: '2026-08-02T00:00:08Z',
      parentContinuationDeliveryPersistedAt: '2026-08-02T00:00:09Z',
      epicContinuationLaunchAcceptedAt: '2026-08-02T00:00:10Z',
    });
    expect(waiting.label).toBe('Waiting for Epic Runner start authorization');
    expect(waiting.providerReceiverActivationObservedAt).toBeUndefined();
    expect(
      phase({
        sprintStartAuthorizedAt: '2026-08-02T00:00:11Z',
        sprintStartPersistedAt: '2026-08-02T00:00:12Z',
      }).label,
    ).toBe('Sprint start authorized; repository reevaluation pending');
    const ready = phase({
      sprintStartPersistedAt: '2026-08-02T00:00:12Z',
      repositoryBranchReevaluationRecordedAt: '2026-08-02T00:00:13Z',
      planningReadyAt: '2026-08-02T00:00:14Z',
      providerReceiverActivationObservedAt: '2026-08-02T00:00:15Z',
    });
    expect(ready.label).toBe('Sprint planning-ready; downstream not started');
    expect(ready.downstreamNotStarted).toBe(true);
    expect(ready.providerReceiverActivationObservedAt).toBe('2026-08-02T00:00:15Z');
  });
});
