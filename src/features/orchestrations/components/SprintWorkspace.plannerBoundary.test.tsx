import { render, screen } from '@testing-library/react';
import {
  composeProductOrchestrationReadModels,
  decodeOrchestrationNativeQueryV2,
  nativeQueryProductCompositionInputV2,
  projectSprintWorkspacePresentation,
  type ProductSprintRunnerTransitionStatusV1,
} from '../../../application/orchestrations';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  SprintRunnerActivationObservation,
  SprintRunnerHandbackActivity,
  SprintContinuationBoundary,
  WorkSlicePlannerBoundary,
} from './SprintWorkspace';

const transition: ProductSprintRunnerTransitionStatusV1 = {
  label: 'Work Slice Planner request authorized; planning point pending',
  requestedAt: '2026-08-02T00:00:00Z',
  authorizedAt: '2026-08-02T00:00:01Z',
  preStartReady: true,
  lifecycleObserved: false,
  accepted: false,
  workSlicePlannerRequestId: 'planner-request-1',
  workSlicePlannerRequestedAt: '2026-08-02T00:00:00Z',
  workSlicePlannerAuthorizedAt: '2026-08-02T00:00:01Z',
};
const sprint = (
  sprintRunnerTransition?: ProductSprintRunnerTransitionStatusV1,
  workUnitMaterializations: readonly unknown[] = [],
) => ({ sprintRunnerTransition, workUnitMaterializations }) as never;

describe('Work Slice Planner boundary disclosure', () => {
  it('presents continuing, attention, settled, and local-result boundaries without higher effects', () => {
    const base = {
      current: { decisionId: 'decision-3', state: 'attention' as const, updatedAt: '2026-08-05T00:00:02Z' },
      history: [
        { decisionId: 'decision-1', sequence: 1, state: 'continuing' as const, reason: 'wait_for_agent_dependency', recordedAt: '2026-08-05T00:00:00Z' },
        {
          decisionId: 'decision-3',
          sequence: 3,
          state: 'attention' as const,
          reason: 'structured_human_or_external_attention',
          recordedAt: '2026-08-05T00:00:02Z',
          attention: {
            code: 'structured_human_or_external_attention',
            structuredAttention: {
              reason: 'Policy decision required.',
              authorityNeeded: 'Product authority',
              evidenceContext: 'Unresolved concern.',
              resumptionPath: 'Resume exact Sprint.',
            },
          },
        },
      ],
      upwardResults: [{ resultId: 'result-3', decisionId: 'decision-3', recordedAt: '2026-08-05T00:00:02Z', resultKind: 'attention' as const }],
    };
    const { rerender } = render(<SprintContinuationBoundary boundary={base} />);
    let region = screen.getByRole('region', { name: 'Sprint continuation boundary' });
    expect(region).toHaveTextContent('Sprint needs attention');
    expect(region).toHaveTextContent('Policy decision required.');
    expect(region).toHaveTextContent('not delivery, Epic receipt, later-Sprint selection, continuation, or acceptance');
    rerender(
      <SprintContinuationBoundary
        boundary={{
          ...base,
          current: { decisionId: 'decision-1', state: 'continuing', updatedAt: '2026-08-05T00:00:00Z' },
        }}
      />,
    );
    region = screen.getByRole('region', { name: 'Sprint continuation boundary' });
    expect(region).toHaveTextContent('Sprint is continuing');
    expect(region).toHaveTextContent('exact agent-achievable dependency route');
    rerender(
      <SprintContinuationBoundary
        boundary={{
          ...base,
          current: { decisionId: 'decision-4', state: 'settled', updatedAt: '2026-08-05T00:00:03Z' },
          history: [{ decisionId: 'decision-4', sequence: 4, state: 'settled', reason: 'all_authoritative_sprint_work_settled', recordedAt: '2026-08-05T00:00:03Z' }],
          upwardResults: [{ resultId: 'result-4', decisionId: 'decision-4', recordedAt: '2026-08-05T00:00:03Z', resultKind: 'settled' }],
        }}
      />,
    );
    region = screen.getByRole('region', { name: 'Sprint continuation boundary' });
    expect(region).toHaveTextContent('Sprint is settled');
    expect(region).toHaveTextContent('does not imply Epic settlement, delivery, a later Sprint, or acceptance');
  });

  it('is absent before a durable Planner request', () => {
    render(<WorkSlicePlannerBoundary sprint={sprint(undefined)} />);
    expect(screen.queryByRole('region', { name: 'Work Slice Planner boundary' })).toBeNull();
  });

  it('is present after a durable Planner request and states the downstream stop', () => {
    render(<WorkSlicePlannerBoundary sprint={sprint(transition)} />);
    expect(screen.getByRole('region', { name: 'Work Slice Planner boundary' })).toHaveTextContent(
      'Proposal facts remain distinct from every later Work Unit or downstream action.',
    );
    expect(screen.getByRole('region', { name: 'Work Slice Planner boundary' })).toHaveTextContent(
      'Planner request',
    );
    expect(screen.getByRole('region', { name: 'Work Slice Planner boundary' })).toHaveTextContent(
      'Planner authorization',
    );
  });

  it('labels proposal lifecycle stages separately without exposing materialization control', () => {
    render(
      <WorkSlicePlannerBoundary
        sprint={sprint({
          ...transition,
          workSliceProposalSubmittedAt: '2026-08-02T00:00:02Z',
          workSliceProposalValidationResult: 'valid',
          workSliceRefinementRequestedAt: '2026-08-02T00:00:03Z',
          workSliceSemanticCompletedAt: undefined,
          workSliceTerminalLifecycleObservedAt: undefined,
          workSliceApplicationAcceptedAt: undefined,
          workSliceMaterializationReadyAt: undefined,
        })}
      />,
    );
    const region = screen.getByRole('region', { name: 'Work Slice Planner boundary' });
    expect(region).toHaveTextContent('Proposal submitted');
    expect(region).toHaveTextContent('Validation accepted');
    expect(region).toHaveTextContent('Refinement requested');
    expect(region).toHaveTextContent('Semantic completion (not recorded)');
    expect(region).toHaveTextContent('Application acceptance (not recorded)');
    expect(region.querySelector('button')).toBeNull();
    expect(region).not.toHaveTextContent('Materialize Work Units');
  });

  it('keeps a settled materialization current without claiming a current downstream stop', () => {
    const settled = [
      {
        materializationId: 'materialization-1',
        planningPointId: 'point-1',
        acceptedRevisionId: 'accepted-revision-1',
        stage: 'settled',
        source: {
          status: 'available',
          sourceKind: 'application_interpretation',
          sourceReferences: ['materialization-1'],
        },
      },
    ];
    render(
      <>
        <SprintRunnerActivationObservation
          transition={{ ...transition, downstreamNotStarted: true }}
          hasCreatedWorkUnits
        />
        <WorkSlicePlannerBoundary sprint={sprint(transition, settled)} />
      </>,
    );
    const region = screen.getByRole('region', { name: 'Work Slice Planner boundary' });
    expect(region).toHaveTextContent('Accepted revision accepted-revision-1');
    expect(region).toHaveTextContent('Work Units and relationships settled');
    expect(region).toHaveTextContent('No Handler activation is recorded.');
    expect(region).not.toHaveTextContent(
      'currently stops at the application-owned Work Slice Planner boundary',
    );
    expect(screen.queryByText('No Work Slice or Work Unit has been created.')).toBeNull();
    expect(document.body).toHaveTextContent(
      'The pre-materialization downstream-not-started record remains historical.',
    );
  });

  it('renders authoritative Handler activity from the native query through the product presentation', () => {
    const value = JSON.parse(
      readFileSync(
        resolve(
          'src-tauri/src/orchestration/fixtures/orchestration-native-query-v2',
          'valid-initiated-epic.json',
        ),
        'utf8',
      ),
    ) as Record<string, unknown>;
    value.workUnitMaterializations = [
      {
        materializationId: 'materialization-1',
        planningPointId: 'point-1',
        acceptedRevisionId: 'accepted-revision-1',
        epicId: 'epic-fixture',
        sprintId: 'sprint-fixture',
        workSliceId: 'slice-1',
        authorizationRecordedAt: '2026-08-02T00:00:00Z',
        attemptRecordedAt: '2026-08-02T00:00:01Z',
        workUnitsCreatedAt: '2026-08-02T00:00:02Z',
        relationshipsCompletedAt: '2026-08-02T00:00:03Z',
        settledAt: '2026-08-02T00:00:04Z',
      },
    ];
    value.workUnits = [
      {
        workUnitId: 'unit-root',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 0,
        laneTitle: 'Root responsibility',
        specification: 'Root specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-root',
          handlerSessionId: 'handler-session-root',
          handlerInvocationId: 'handler-invocation-root',
          eligibilityState: 'eligible',
          requestedAt: '2026-08-02T00:01:00Z',
          authorizedAt: '2026-08-02T00:01:01Z',
          attemptCreatedAt: '2026-08-02T00:01:02Z',
          executionSupportGrantedAt: '2026-08-02T00:01:03Z',
          isolatedWorktreeReadyAt: '2026-08-02T00:01:04Z',
          handlerSessionCreatedAt: '2026-08-02T00:01:05Z',
          handlerInvocationPreparedAt: '2026-08-02T00:01:06Z',
          handlerHarnessBoundAt: '2026-08-02T00:01:07Z',
          launchRequestedAt: '2026-08-02T00:01:08Z',
          launchAcceptedAt: '2026-08-02T00:01:09Z',
          handlerReadyAt: '2026-08-02T00:01:10Z',
          providerActivationObservedAt: '2026-08-02T00:01:11Z',
        },
      },
      {
        workUnitId: 'unit-dependent',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 1,
        laneTitle: 'Dependent responsibility',
        specification: 'Dependent specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-dependent',
          eligibilityState: 'blocked',
          blockedReason: 'prerequisite_satisfaction_not_authoritative',
          requestedAt: '2026-08-02T00:01:00Z',
        },
      },
      {
        workUnitId: 'unit-descriptive-handler',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 2,
        laneTitle: 'Handler is only descriptive text',
        specification: 'This specification mentions Handler but has no activation.',
      },
      {
        workUnitId: 'unit-eligible',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 3,
        laneTitle: 'Eligible responsibility',
        specification: 'Eligible specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-eligible',
          eligibilityState: 'eligible',
          requestedAt: '2026-08-02T00:01:00Z',
        },
      },
      {
        workUnitId: 'unit-prepared',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 4,
        laneTitle: 'Prepared responsibility',
        specification: 'Prepared specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-prepared',
          eligibilityState: 'eligible',
          requestedAt: '2026-08-02T00:01:00Z',
          authorizedAt: '2026-08-02T00:01:01Z',
          attemptCreatedAt: '2026-08-02T00:01:02Z',
          executionSupportGrantedAt: '2026-08-02T00:01:03Z',
          isolatedWorktreeReadyAt: '2026-08-02T00:01:04Z',
          handlerSessionCreatedAt: '2026-08-02T00:01:05Z',
          handlerInvocationPreparedAt: '2026-08-02T00:01:06Z',
        },
      },
      {
        workUnitId: 'unit-requested',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 5,
        laneTitle: 'Launch-requested responsibility',
        specification: 'Launch-requested specification.',
        handlerActivation: {
          attemptId: 'handler-attempt-requested',
          eligibilityState: 'eligible',
          requestedAt: '2026-08-02T00:01:00Z',
          authorizedAt: '2026-08-02T00:01:01Z',
          attemptCreatedAt: '2026-08-02T00:01:02Z',
          executionSupportGrantedAt: '2026-08-02T00:01:03Z',
          isolatedWorktreeReadyAt: '2026-08-02T00:01:04Z',
          handlerSessionCreatedAt: '2026-08-02T00:01:05Z',
          handlerInvocationPreparedAt: '2026-08-02T00:01:06Z',
          handlerHarnessBoundAt: '2026-08-02T00:01:07Z',
          launchRequestedAt: '2026-08-02T00:01:08Z',
        },
      },
    ];
    value.workUnits = (value.workUnits as Array<Record<string, unknown>>).map((workUnit) => ({
      ...workUnit,
      attemptHistory: [],
      retryAttempts: [],
    }));
    value.workUnitRelationships = [
      {
        relationshipId: 'point',
        materializationId: 'materialization-1',
        relationshipKind: 'planning_point',
        fromId: 'point-1',
        toId: 'slice-1',
      },
      {
        relationshipId: 'sprint',
        materializationId: 'materialization-1',
        relationshipKind: 'sprint',
        fromId: 'sprint-fixture',
        toId: 'slice-1',
      },
      ...[
        'unit-root',
        'unit-dependent',
        'unit-descriptive-handler',
        'unit-eligible',
        'unit-prepared',
        'unit-requested',
      ].flatMap((workUnitId, ordinal) => [
        {
          relationshipId: `lane-${ordinal}`,
          materializationId: 'materialization-1',
          relationshipKind: 'lane',
          fromId: 'slice-1',
          toId: workUnitId,
          ordinal,
        },
        {
          relationshipId: `order-${ordinal}`,
          materializationId: 'materialization-1',
          relationshipKind: 'order',
          fromId: 'slice-1',
          toId: workUnitId,
          ordinal,
        },
      ]),
      {
        relationshipId: 'dependency',
        materializationId: 'materialization-1',
        relationshipKind: 'depends_on',
        fromId: 'unit-dependent',
        toId: 'unit-root',
      },
    ];
    value.dependencyActivationIntents = [
      {
        workUnitId: 'unit-root',
        materializationId: 'materialization-1',
        acceptedRevisionId: 'accepted-revision-1',
        eligibilityState: 'eligible',
        eligibilityRecordedAt: '2026-08-02T00:01:12Z',
        activationIntendedAt: '2026-08-02T00:01:13Z',
      },
      {
        workUnitId: 'unit-dependent',
        materializationId: 'materialization-1',
        acceptedRevisionId: 'accepted-revision-1',
        eligibilityState: 'blocked',
        blockedReason: 'missing_prerequisite_contributions:dependency',
        eligibilityRecordedAt: '2026-08-02T00:01:12Z',
        activationIntendedAt: '2026-08-02T00:01:13Z',
      },
    ];
    const workspace = projectSprintWorkspacePresentation(
      composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(value)),
      ).epics[0]!.sprints[0]!,
    );
    const workUnits = workspace.revisionViews[0]!.workUnits.map((workUnit) =>
      workUnit.workUnitId === 'unit-descriptive-handler'
        ? { ...workUnit, details: 'Handler appears only in display prose.' }
        : workUnit,
    );
    expect(
      workUnits.find(({ workUnitId }) => workUnitId === 'unit-descriptive-handler'),
    ).not.toHaveProperty('handlerActivation');
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-root')).toMatchObject({
      handlerActivation: {
        eligibilityState: 'eligible',
        stage: 'handler_ready',
        providerActivityObserved: true,
      },
    });
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-dependent')).toMatchObject({
      dependencyActivationIntent: {
        eligibilityState: 'blocked',
        blockedReason: 'missing_prerequisite_contributions:dependency',
        activationIntendedAt: '2026-08-02T00:01:13Z',
      },
      handlerActivation: {
        eligibilityState: 'blocked',
        blockedReason: 'prerequisite_satisfaction_not_authoritative',
      },
    });
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-eligible')).toMatchObject({
      handlerActivation: { eligibilityState: 'eligible', stage: 'eligible_not_prepared' },
    });
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-prepared')).toMatchObject({
      handlerActivation: { eligibilityState: 'eligible', stage: 'invocation_prepared' },
    });
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-requested')).toMatchObject({
      handlerActivation: { eligibilityState: 'eligible', stage: 'launch_requested' },
    });
    expect(workUnits.find(({ workUnitId }) => workUnitId === 'unit-root')).toMatchObject({
      dependencyActivationIntent: {
        eligibilityState: 'eligible',
        activationIntendedAt: '2026-08-02T00:01:13Z',
      },
    });

    render(
      <WorkSlicePlannerBoundary
        sprint={{ ...workspace.sprint, sprintRunnerTransition: transition }}
        workUnits={workUnits}
      />,
    );

    const activity = screen.getByRole('region', { name: 'Handler activation activity' });
    expect(activity).toHaveTextContent('Root responsibility');
    expect(activity).toHaveTextContent(
      'Handler launch accepted and application Handler readiness recorded.',
    );
    expect(activity).toHaveTextContent('Provider activity observed separately');
    expect(activity).toHaveTextContent('Dependent responsibility');
    expect(activity).toHaveTextContent(
      'Dependency activation blocked: missing_prerequisite_contributions:dependency.',
    );
    expect(activity).toHaveTextContent(
      'Root responsibility: Dependencies eligible; Handler activation intent recorded.',
    );
    expect(activity).toHaveTextContent(
      'Handler activation blocked: prerequisite_satisfaction_not_authoritative.',
    );
    expect(activity).toHaveTextContent(
      'Eligible responsibility: Handler activation is eligible but not yet prepared.',
    );
    expect(activity).toHaveTextContent(
      'Prepared responsibility: Handler invocation prepared; launch is not yet recorded.',
    );
    expect(activity).toHaveTextContent(
      'Launch-requested responsibility: Handler launch requested; acceptance is not yet recorded.',
    );
    expect(activity).not.toHaveTextContent('Handler is only descriptive text');
    expect(activity).not.toHaveTextContent(
      /Implementer|Handler review|implementation output|retry attempt|application acceptance|Sprint continuation/,
    );
  });
});

describe('Sprint Runner Handback disclosure', () => {
  it('treats omitted history in an established recorded presentation input as no Handback activity', () => {
    render(
      <SprintRunnerHandbackActivity
        workUnits={[
          {
            workUnitId: 'recorded-unit-without-history',
            title: 'Established recorded responsibility',
          },
        ] as never}
      />,
    );

    expect(screen.queryByRole('region', { name: 'Sprint Runner Handback reassessment' })).toBeNull();
  });

  it('keeps a Handback and delivery partial state factual without inventing movement or progress', () => {
    render(
      <SprintRunnerHandbackActivity
        workUnits={[
          {
            workUnitId: 'unit-partial-handback',
            title: 'Partial Handback concern',
            attemptHistory: [
              {
                ordinal: 0,
                attemptId: 'attempt-partial-handback',
                incompleteDisposition: {
                  attemptId: 'attempt-partial-handback',
                  reviewInvocationId: 'review-partial-handback',
                  decisionFingerprint: 'decision-partial-handback',
                  classification: 'blocked',
                  meaningfulProgress: false,
                  recordedAt: '2026-08-04T00:00:00Z',
                  noProgressHandback: {
                    handbackId: 'handback-partial-handback',
                    sourceAttemptId: 'attempt-partial-handback',
                    sourceReviewInvocationId: 'review-partial-handback',
                    contextFingerprint: 'context-partial-handback',
                    persistedAt: '2026-08-04T00:00:01Z',
                    deliveryIntendedAt: '2026-08-04T00:00:02Z',
                    sprintRunnerDelivery: {
                      deliveryRequestedAt: '2026-08-04T00:00:03Z',
                    },
                  },
                },
              },
            ],
          },
        ] as never}
      />,
    );
    const region = screen.getByRole('region', { name: 'Sprint Runner Handback reassessment' });
    expect(region).toHaveTextContent('The handed-back concern remains unresolved.');
    expect(region).toHaveTextContent('Only recorded Handback and Sprint Runner stages are shown.');
    expect(region).toHaveTextContent('Delivery requested at');
    expect(region).not.toHaveTextContent('recorded next movement proceeds');
    expect(region).not.toHaveTextContent('Selected movement recorded');
    expect(region).not.toHaveTextContent('meaningful progress');
    expect(region).not.toHaveTextContent('Local exhaustion recorded');
    expect(region).toHaveTextContent('no Epic response is recorded here');
  });

  it('shows factual stages and qualified movement without final blockage or Epic response', () => {
    render(
      <SprintRunnerHandbackActivity
        workUnits={
          [
            {
              workUnitId: 'unit-handback',
              title: 'Handed-back concern',
              attemptHistory: [
                {
                  ordinal: 0,
                  attemptId: 'attempt-1',
                  incompleteDisposition: {
                    attemptId: 'attempt-1',
                    reviewInvocationId: 'review-1',
                    decisionFingerprint: 'decision-1',
                    classification: 'blocked',
                    meaningfulProgress: false,
                    recordedAt: '2026-08-04T00:00:00Z',
                    noProgressHandback: {
                      handbackId: 'handback-1',
                      sourceAttemptId: 'attempt-1',
                      sourceReviewInvocationId: 'review-1',
                      contextFingerprint: 'context-1',
                      persistedAt: '2026-08-04T00:00:01Z',
                      deliveryIntendedAt: '2026-08-04T00:00:02Z',
                      sprintRunnerDelivery: {
                        deliveryRequestedAt: '2026-08-04T00:00:03Z',
                        deliveryPersistedAt: '2026-08-04T00:00:04Z',
                        harnessBoundAt: '2026-08-04T00:00:05Z',
                        launchRequestedAt: '2026-08-04T00:00:06Z',
                        launchAcceptedAt: '2026-08-04T00:00:07Z',
                        semanticReassessmentRecordedAt: '2026-08-04T00:00:08Z',
                        selectedMovementKind: 'wait_for_agent_dependency',
                        selectedMovement: {
                          movementKind: 'wait_for_agent_dependency',
                          rationale: 'The concern remains open.',
                          dependencyOwner: 'bounded Work Unit Handler',
                          dependencyOwnerClassification: 'work_unit_handler',
                          enablingResult: 'A persisted Handler result.',
                          resumptionPath: 'Reconcile this exact Handback.',
                        },
                      },
                    },
                  },
                },
              ],
            },
          ] as never
        }
      />,
    );
    const region = screen.getByRole('region', { name: 'Sprint Runner Handback reassessment' });
    expect(region).toHaveTextContent('The handed-back concern remains unresolved');
    expect(region).toHaveTextContent('Delivery persisted');
    expect(region).toHaveTextContent('Semantic reassessment recorded');
    expect(region).toHaveTextContent('Agent-achievable dependency wait');
    expect(region).toHaveTextContent('enabling result: A persisted Handler result.');
    expect(region).toHaveTextContent('resumption path: Reconcile this exact Handback.');
    expect(region).toHaveTextContent('not final Sprint or Epic blockage');
    expect(region).toHaveTextContent('no Epic response is recorded here');
  });

  it('separates local exhaustion intent from the later delivery request and keeps bounded movement neutral', () => {
    render(
      <SprintRunnerHandbackActivity
        workUnits={[
          {
            workUnitId: 'unit-local-exhaustion',
            title: 'Concern remains open',
            attemptHistory: [
              {
                ordinal: 0,
                attemptId: 'attempt-local-exhaustion',
                incompleteDisposition: {
                  attemptId: 'attempt-local-exhaustion',
                  reviewInvocationId: 'review-local-exhaustion',
                  decisionFingerprint: 'decision-local-exhaustion',
                  classification: 'blocked',
                  meaningfulProgress: false,
                  recordedAt: '2026-08-04T00:00:00Z',
                  noProgressHandback: {
                    handbackId: 'handback-local-exhaustion',
                    sourceAttemptId: 'attempt-local-exhaustion',
                    sourceReviewInvocationId: 'review-local-exhaustion',
                    contextFingerprint: 'context-local-exhaustion',
                    persistedAt: '2026-08-04T00:00:01Z',
                    deliveryIntendedAt: '2026-08-04T00:00:02Z',
                    sprintRunnerDelivery: {
                      deliveryRequestedAt: '2026-08-04T00:00:03Z',
                      semanticReassessmentRecordedAt: '2026-08-04T00:00:04Z',
                      selectedMovementKind: 'local_exhaustion_escalate',
                      selectedMovement: {
                        movementKind: 'local_exhaustion_escalate',
                        rationale: 'No further local movement is recorded.',
                        localExhaustionSummary: 'The concern remains unresolved locally.',
                      },
                      escalationIntentRecordedAt: '2026-08-04T00:00:05Z',
                      escalationDeliveryRequestedAt: '2026-08-04T00:00:06Z',
                    },
                  },
                },
              },
            ],
          },
        ] as never}
      />,
    );
    const region = screen.getByRole('region', { name: 'Sprint Runner Handback reassessment' });
    expect(region).toHaveTextContent('Local exhaustion recorded');
    expect(region).toHaveTextContent('Escalation intent recorded upward');
    expect(region).toHaveTextContent('Escalation delivery request recorded upward');
    expect(region).not.toHaveTextContent('Escalation delivered');
    expect(region).toHaveTextContent('not final Sprint or Epic blockage');
    expect(region).not.toHaveTextContent('is final Sprint or Epic blockage');
    expect(region).not.toHaveTextContent('Epic response recorded');
  });

  it('labels an extensible bounded movement without implying progress or settlement', () => {
    render(
      <SprintRunnerHandbackActivity
        workUnits={[
          {
            workUnitId: 'unit-bounded-movement',
            title: 'Bounded movement concern',
            attemptHistory: [
              {
                ordinal: 0,
                attemptId: 'attempt-bounded-movement',
                incompleteDisposition: {
                  attemptId: 'attempt-bounded-movement',
                  reviewInvocationId: 'review-bounded-movement',
                  decisionFingerprint: 'decision-bounded-movement',
                  classification: 'blocked',
                  meaningfulProgress: false,
                  recordedAt: '2026-08-04T00:00:00Z',
                  noProgressHandback: {
                    handbackId: 'handback-bounded-movement',
                    sourceAttemptId: 'attempt-bounded-movement',
                    sourceReviewInvocationId: 'review-bounded-movement',
                    contextFingerprint: 'context-bounded-movement',
                    persistedAt: '2026-08-04T00:00:01Z',
                    deliveryIntendedAt: '2026-08-04T00:00:02Z',
                    sprintRunnerDelivery: {
                      deliveryRequestedAt: '2026-08-04T00:00:03Z',
                      semanticReassessmentRecordedAt: '2026-08-04T00:00:04Z',
                      selectedMovementKind: 'future_bounded_move',
                      selectedMovement: {
                        movementKind: 'future_bounded_move',
                        rationale: 'The concern remains open.',
                        boundedDetails: [
                          { label: 'dependencyOwner', value: 'owner-shaped detail only' },
                          { label: 'dependencyOwnerClassification', value: 'work_unit_handler' },
                          { label: 'eligibleWorkSummary', value: 'alternate-shaped detail only' },
                        ],
                      },
                    },
                  },
                },
              },
            ],
          },
        ] as never}
      />,
    );
    const region = screen.getByRole('region', { name: 'Sprint Runner Handback reassessment' });
    expect(region).toHaveTextContent('Bounded movement recorded: The concern remains open.');
    expect(region).toHaveTextContent('owner-shaped detail only');
    expect(region).toHaveTextContent('work_unit_handler');
    expect(region).toHaveTextContent('alternate-shaped detail only');
    expect(region).not.toHaveTextContent('Agent-achievable dependency wait');
    expect(region).toHaveTextContent('no settlement or blockage is implied');
  });
});
