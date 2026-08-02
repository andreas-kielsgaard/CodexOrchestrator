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
import { SprintRunnerActivationObservation, WorkSlicePlannerBoundary } from './SprintWorkspace';

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
          handlerInvocationPreparedAt: '2026-08-02T00:01:06Z',
          launchRequestedAt: '2026-08-02T00:01:08Z',
        },
      },
    ];
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
