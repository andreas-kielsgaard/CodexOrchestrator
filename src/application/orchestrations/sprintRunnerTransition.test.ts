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
      startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:13Z',
      planningReadyAt: '2026-08-02T00:00:14Z',
      providerReceiverActivationObservedAt: '2026-08-02T00:00:15Z',
    });
    expect(ready.label).toBe('Sprint planning-ready; downstream not started');
    expect(ready.downstreamNotStarted).toBe(true);
    expect(ready.providerReceiverActivationObservedAt).toBe('2026-08-02T00:00:15Z');
  });

  it('maps the durable Work Slice Planner boundary and keeps later observations absent', () => {
    const value = query();
    Object.assign(value.transitions[0]!, {
      workSlicePlannerRequestId: 'planner-request-1',
      workSlicePlannerRequestedAt: '2026-08-02T00:00:15Z',
      workSlicePlannerAuthorizedAt: '2026-08-02T00:00:15Z',
      startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:14Z',
      planningControlLaunchAcceptedAt: '2026-08-02T00:00:14Z',
      workSlicePlanningPointId: 'planning-point-1',
      workSlicePlannerRepositoryWorktreeRoute: 'C:/authority/worktree',
      workSlicePlannerHarnessKey: 'planner-harness',
      workSlicePlannerHarnessVersion: 3,
      workSlicePlannerSessionId: 'planner-session-1',
      workSlicePlannerInvocationId: 'planner-invocation-1',
      workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:16Z',
      workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:17Z',
      workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:18Z',
      workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:19Z',
      workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:20Z',
      workSlicePlannerReadyAt: '2026-08-02T00:00:21Z',
      workSlicePlannerProviderActivationObservedAt: null,
      workSlicePlannerLifecycleObservedAt: null,
    });
    const transition = decodeSprintRunnerTransitionQueryV1(value).transitions[0]!;
    expect(projectSprintRunnerTransitionStatus(transition)).toMatchObject({
      workSlicePlannerRequestId: 'planner-request-1',
      workSlicePlannerHarnessVersion: 3,
      workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:20Z',
      workSlicePlannerReadyAt: '2026-08-02T00:00:21Z',
    });
    expect(transition.workSlicePlannerProviderActivationObservedAt).toBeUndefined();
    expect(transition.workSlicePlannerLifecycleObservedAt).toBeUndefined();
  });

  it('projects meaningful Planner stages progressively and rejects skipped ordering', () => {
    const planner = (fields: Record<string, unknown>) => {
      const value = query();
      Object.assign(value.transitions[0]!, {
        startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:10Z',
        planningControlLaunchAcceptedAt: '2026-08-02T00:00:11Z',
        workSlicePlannerRequestId: 'planner-request-1',
        workSlicePlannerRequestedAt: '2026-08-02T00:00:11Z',
        workSlicePlannerAuthorizedAt: '2026-08-02T00:00:11Z',
        workSlicePlanningPointId: 'planning-point-1',
        workSlicePlannerSessionId: 'reserved-session-1',
        workSlicePlannerInvocationId: 'reserved-invocation-1',
        ...fields,
      });
      return projectSprintRunnerTransitionStatus(
        decodeSprintRunnerTransitionQueryV1(value).transitions[0]!,
      );
    };
    expect(planner({}).label).toBe('Work Slice Planner planning point created; Session pending');
    const requestOnly = query();
    Object.assign(requestOnly.transitions[0]!, {
      startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:10Z',
      planningControlLaunchAcceptedAt: '2026-08-02T00:00:11Z',
      workSlicePlannerRequestId: 'planner-request-1',
      workSlicePlannerRequestedAt: '2026-08-02T00:00:12Z',
    });
    expect(
      projectSprintRunnerTransitionStatus(
        decodeSprintRunnerTransitionQueryV1(requestOnly).transitions[0]!,
      ).label,
    ).toBe('Work Slice Planner request recorded; authorization pending');
    const skippedAuthorization = query();
    Object.assign(skippedAuthorization.transitions[0]!, {
      startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:10Z',
      planningControlLaunchAcceptedAt: '2026-08-02T00:00:11Z',
      workSlicePlannerRequestId: 'planner-request-1',
      workSlicePlannerRequestedAt: '2026-08-02T00:00:12Z',
      workSlicePlanningPointId: 'planning-point-1',
    });
    expect(() => decodeSprintRunnerTransitionQueryV1(skippedAuthorization)).toThrow(
      'requires durable authorization',
    );
    expect(planner({ workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z' }).label).toBe(
      'Work Slice Planner Session created; invocation pending',
    );
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
      }).label,
    ).toBe('Work Slice Planner invocation prepared; Harness application pending');
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:12Z',
      }).label,
    ).toBe('Work Slice Planner Harness applied; launch request pending');
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:13Z',
      }).label,
    ).toBe('Work Slice Planner launch requested; runtime acceptance pending');
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:14Z',
      }).label,
    ).toBe('Work Slice Planner runtime launch accepted; readiness pending');
    const ready = planner({
      workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
      workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
      workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:12Z',
      workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:13Z',
      workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:14Z',
      workSlicePlannerReadyAt: '2026-08-02T00:00:15Z',
      workSlicePlannerProviderActivationObservedAt: '2026-08-02T00:00:16Z',
      workSlicePlannerLifecycleObservedAt: '2026-08-02T00:00:17Z',
    });
    expect(ready.label).toBe('Work Slice Planner ready; provider and lifecycle observations recorded');
    expect(ready.workSlicePlannerProviderActivationObservedAt).toBe('2026-08-02T00:00:16Z');
    expect(ready.workSlicePlannerLifecycleObservedAt).toBe('2026-08-02T00:00:17Z');
    const readyWithoutObservations = planner({
      workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
      workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
      workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:14Z',
      workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:15Z',
      workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:16Z',
      workSlicePlannerReadyAt: '2026-08-02T00:00:17Z',
    });
    expect(readyWithoutObservations.label).toBe(
      'Work Slice Planner ready; provider and lifecycle observation pending',
    );
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:14Z',
        workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:15Z',
        workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:16Z',
        workSlicePlannerReadyAt: '2026-08-02T00:00:17Z',
        workSlicePlannerProviderActivationObservedAt: '2026-08-02T00:00:18Z',
      }).label,
    ).toBe('Work Slice Planner ready; provider observed; lifecycle observation pending');
    expect(
      planner({
        workSlicePlannerSessionCreatedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerInvocationCreatedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:14Z',
        workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:15Z',
        workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:16Z',
        workSlicePlannerReadyAt: '2026-08-02T00:00:17Z',
        workSlicePlannerLifecycleObservedAt: '2026-08-02T00:00:18Z',
      }).label,
    ).toBe('Work Slice Planner ready; lifecycle observed; provider activation pending');
    expect(() => planner({ workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:14Z' })).toThrow(
      'launch acceptance requires requested launch',
    );
    expect(() => planner({ workSlicePlannerReadyAt: '2026-08-02T00:00:15Z' })).toThrow(
      'readiness requires accepted launch',
    );
  });

  it('rejects acceptance or readiness for invalid, incomplete, or refinement-pending revisions', () => {
    const proposal = (fields: Record<string, unknown>) => {
      const value = query();
      Object.assign(value.transitions[0]!, {
        startedReevaluationLifecycleObservedAt: '2026-08-02T00:00:10Z',
        planningControlLaunchAcceptedAt: '2026-08-02T00:00:11Z',
        workSlicePlannerRequestId: 'planner-request-1',
        workSlicePlannerRequestedAt: '2026-08-02T00:00:11Z',
        workSlicePlannerAuthorizedAt: '2026-08-02T00:00:11Z',
        workSlicePlanningPointId: 'planning-point-1',
        workSlicePlannerSessionId: 'planner-session-1',
        workSlicePlannerInvocationId: 'planner-invocation-1',
        workSlicePlannerHarnessAppliedAt: '2026-08-02T00:00:12Z',
        workSlicePlannerLaunchRequestedAt: '2026-08-02T00:00:13Z',
        workSlicePlannerLaunchAcceptedAt: '2026-08-02T00:00:14Z',
        workSlicePlannerReadyAt: '2026-08-02T00:00:15Z',
        workSliceProposalSubmittedAt: '2026-08-02T00:00:16Z',
        ...fields,
      });
      return value;
    };
    expect(() => decodeSprintRunnerTransitionQueryV1(proposal({
      workSliceProposalValidationResult: 'invalid',
      workSliceRefinementRequestedAt: '2026-08-02T00:00:17Z',
    }))).toThrow('refinement requires a valid proposal');
    expect(() => decodeSprintRunnerTransitionQueryV1(proposal({
      workSliceProposalValidationResult: 'valid',
      workSliceTerminalLifecycleObservedAt: '2026-08-02T00:00:18Z',
      workSliceApplicationAcceptedAt: '2026-08-02T00:00:19Z',
    }))).toThrow('proposal lifecycle observation requires semantic completion');
    expect(() => decodeSprintRunnerTransitionQueryV1(proposal({
      workSliceProposalValidationResult: 'invalid',
      workSliceSemanticCompletedAt: '2026-08-02T00:00:17Z',
    }))).toThrow('semantic completion requires current valid unrefined proposal');
    const coherent = decodeSprintRunnerTransitionQueryV1(proposal({
      workSliceProposalValidationResult: 'valid',
      workSliceSemanticCompletedAt: '2026-08-02T00:00:17Z',
      workSliceTerminalLifecycleObservedAt: '2026-08-02T00:00:18Z',
      workSliceApplicationAcceptedAt: '2026-08-02T00:00:19Z',
      workSliceMaterializationReadyAt: '2026-08-02T00:00:20Z',
    }));
    expect(projectSprintRunnerTransitionStatus(coherent.transitions[0]!).accepted).toBe(false);
  });
});
