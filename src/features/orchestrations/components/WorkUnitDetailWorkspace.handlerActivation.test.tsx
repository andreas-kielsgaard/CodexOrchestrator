import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen } from '@testing-library/react';
import {
  composeProductOrchestrationReadModels,
  decodeOrchestrationNativeQueryV2,
  nativeQueryProductCompositionInputV2,
  projectSprintWorkspacePresentation,
} from '../../../application/orchestrations';
import { WorkUnitDetailWorkspace } from './WorkUnitDetailWorkspace';

describe('WorkUnitDetailWorkspace Handler activation detail', () => {
  it('shows launch acceptance without inventing Handler readiness', () => {
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
        workUnitId: 'unit-launch-accepted',
        materializationId: 'materialization-1',
        workSliceId: 'slice-1',
        acceptedRevisionId: 'accepted-revision-1',
        laneOrdinal: 0,
        laneTitle: 'Launch accepted responsibility',
        specification: 'Launch acceptance is separate from Handler readiness.',
        handlerActivation: {
          attemptId: 'handler-attempt-accepted',
          handlerSessionId: 'handler-session-accepted',
          handlerInvocationId: 'handler-invocation-accepted',
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
        },
        actionContinuation: {
          attemptId: 'handler-attempt-accepted',
          handlerSessionId: 'handler-session-accepted',
          originalHandlerInvocationId: 'handler-invocation-accepted',
          actionInvocationId: 'handler-action-invocation-accepted',
          actionHarnessRevisionId: 'handler-action-revision',
          requestedAt: '2026-08-02T00:01:10Z',
          blockedReason: 'original_handler_invocation_active',
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
      {
        relationshipId: 'lane',
        materializationId: 'materialization-1',
        relationshipKind: 'lane',
        fromId: 'slice-1',
        toId: 'unit-launch-accepted',
        ordinal: 0,
      },
      {
        relationshipId: 'order',
        materializationId: 'materialization-1',
        relationshipKind: 'order',
        fromId: 'slice-1',
        toId: 'unit-launch-accepted',
        ordinal: 0,
      },
    ];
    value.dependencyActivationIntents = [
      {
        workUnitId: 'unit-launch-accepted',
        materializationId: 'materialization-1',
        acceptedRevisionId: 'accepted-revision-1',
        eligibilityState: 'eligible',
        eligibilityRecordedAt: '2026-08-02T00:00:05Z',
        activationIntendedAt: '2026-08-02T00:00:06Z',
      },
    ];

    const workspace = projectSprintWorkspacePresentation(
      composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(decodeOrchestrationNativeQueryV2(value)),
      ).epics[0]!.sprints[0]!,
    );
    const unit = workspace.revisionViews[0]!.workUnits[0]!;
    expect(unit.handlerActivation).toMatchObject({
      eligibilityState: 'eligible',
      stage: 'launch_accepted',
      providerActivityObserved: false,
    });

    const rendered = render(
      <WorkUnitDetailWorkspace
        unit={unit}
        lifecycleEntries={[]}
        workSlicePlanningPointGroupTitle="Planning point"
        sessions={[]}
        onBack={vi.fn()}
      />,
    );

    const detail = screen.getByLabelText('Work Unit context');
    expect(
      screen.getByLabelText('Work Unit activation activity').querySelector('button'),
    ).toBeNull();
    expect(detail).toHaveTextContent(
      'Handler launch accepted; application Handler readiness is not yet recorded.',
    );
    expect(detail).toHaveTextContent(
      'Dependencies are eligible and Handler activation intent is durably recorded.',
    );
    expect(detail).not.toHaveTextContent('acceptance is not yet recorded');
    expect(detail).toHaveTextContent(
      'Handler action continuation is blocked: original_handler_invocation_active.',
    );
    expect(detail).not.toHaveTextContent(
      /provider compliance|outcome|review|retry|application acceptance/,
    );

    rendered.rerender(
      <WorkUnitDetailWorkspace
        unit={{
          ...unit,
          actionContinuation: {
            stage: 'failed',
            failureReason: 'handler_action_launch_not_accepted',
            providerActivityObserved: false,
          },
        }}
        lifecycleEntries={[]}
        workSlicePlanningPointGroupTitle="Planning point"
        sessions={[]}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Handler action continuation needs attention: handler_action_launch_not_accepted.',
    );

    value.dependencyActivationIntents = [
      {
        workUnitId: 'unit-launch-accepted',
        materializationId: 'materialization-1',
        acceptedRevisionId: 'accepted-revision-1',
        eligibilityState: 'blocked',
        blockedReason: 'missing_prerequisite_contributions:edge-1',
        eligibilityRecordedAt: '2026-08-02T00:00:07Z',
        activationIntendedAt: '2026-08-02T00:00:06Z',
      },
    ];
    const blockedQuery = decodeOrchestrationNativeQueryV2(value);
    const blockedUnit = projectSprintWorkspacePresentation(
      composeProductOrchestrationReadModels(
        nativeQueryProductCompositionInputV2(blockedQuery),
      ).epics[0]!.sprints[0]!,
    ).revisionViews[0]!.workUnits[0]!;
    expect(blockedUnit.dependencyActivationIntent).toMatchObject({
      eligibilityState: 'blocked',
      blockedReason: 'missing_prerequisite_contributions:edge-1',
      activationIntendedAt: '2026-08-02T00:00:06Z',
    });
    value.dependencyActivationIntents = [
      { ...(value.dependencyActivationIntents as Array<Record<string, unknown>>)[0], workUnitId: 'foreign-unit' },
    ];
    expect(() => decodeOrchestrationNativeQueryV2(value)).toThrow(
      'invalid Work Unit/materialization/revision correlation',
    );
  });
});
