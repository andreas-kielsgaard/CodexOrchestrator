import { fireEvent, render, screen, within } from '@testing-library/react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type {
  SprintAgentSessionPresentation,
  WorkUnitAgentSessionPresentation,
} from '../orchestrationModel';
import { WorkSlicePlanningPointDetailWorkspace } from './WorkSlicePlanningPointDetailWorkspace';

const source = {
  status: 'available' as const,
  sourceKind: 'orchestration_event' as const,
  sourceReferences: ['recorded-plan'],
};

const workSlicePlanningPointGroup: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number] =
  {
    workSlicePlanningPointId: 'plan-parallel',
    title: 'Parallel planning',
    purpose: 'Coordinate two planning conversations.',
    source,
    membershipSource: source,
    workUnitScopeIds: ['scope-1'],
  };

const session: SprintAgentSessionPresentation = {
  sessionId: 'planner-primary',
  title: 'Work Slice Planner',
};

const unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number] = {
  workUnitId: 'WU-1',
  title: 'Typed relationship baseline',
  summary: 'Show application-owned relationships.',
  details: 'The current planning point owns this Work Unit.',
  source,
  workUnitScopeId: 'scope-1',
  sprintPlanRevisionId: 'revision-1',
  fixedExecutionScopeIds: ['execution-1'],
  dependencies: [],
  gateIds: [],
  attempts: [{ attemptId: 'attempt-1', workUnitExecutionId: 'execution-1', returned: true }],
  reviews: [],
  observed: {
    executionRequested: true,
    launched: true,
    returned: true,
    integrated: false,
    responsibilityAccepted: false,
  },
  presentationState: 'returned',
};

const handler: WorkUnitAgentSessionPresentation = {
  sessionId: 'handler-1',
  title: 'Typed Handler',
  workUnitId: 'WU-1',
  role: 'handler',
};

const implementer: WorkUnitAgentSessionPresentation = {
  sessionId: 'implementer-1',
  title: 'Typed Implementer',
  workUnitId: 'WU-1',
  role: 'implementer',
};

describe('WorkSlicePlanningPointDetailWorkspace', () => {
  it('shows the typed Planner-to-Work-Unit baseline without a recorded workflow adjunct', () => {
    const onOpenWorkUnit = vi.fn();
    render(
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={workSlicePlanningPointGroup}
        currentWorkState="Processing"
        workUnitRelationships={[
          { workUnit: unit, handlers: [handler], implementers: [implementer] },
        ]}
        plannerSession={session}
        onBack={vi.fn()}
        onOpenWorkUnit={onOpenWorkUnit}
      />,
    );

    expect(screen.getByLabelText('Work Slice planning point controls')).toHaveTextContent(
      'Current workProcessing',
    );
    const timeline = screen.getByLabelText('Work Slice causal timeline');
    expect(screen.getByLabelText('Work Slice Planner origin')).toHaveTextContent(
      'Work Slice PlannerWork Slice Planner',
    );
    const workUnit = within(timeline).getByRole('article', { name: /Work Unit WU-1/ });
    expect(workUnit).toHaveTextContent('WU-1Typed relationship baseline');
    expect(
      within(workUnit).getByRole('button', { name: /Open WU-1 Handler lifecycle/ }),
    ).toHaveTextContent('Handler: Typed Handler');
    expect(
      within(workUnit).getByRole('button', { name: /Open WU-1 Worker lifecycle/ }),
    ).toHaveTextContent('Worker: Typed Implementer');
    fireEvent.click(within(workUnit).getByRole('button', { name: 'Open Work Unit' }));
    expect(onOpenWorkUnit).toHaveBeenCalledWith('WU-1');
    expect(screen.queryByLabelText('Detailed workflow unavailable')).toBeNull();
    expect(screen.queryByText(/historical Plan/)).toBeNull();
    expect(screen.getByLabelText('Work Slice Planner Agent Session')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Collapse Agent Session' })).toBeNull();
  });

  it('keeps a typed Work Unit visible when its Handler and Implementer are unavailable', () => {
    render(
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={workSlicePlanningPointGroup}
        currentWorkState="Planned"
        workUnitRelationships={[{ workUnit: unit, handlers: [], implementers: [] }]}
        plannerSession={session}
        onBack={vi.fn()}
        onOpenWorkUnit={vi.fn()}
      />,
    );

    expect(screen.getByRole('article', { name: /Work Unit WU-1/ })).toBeVisible();
    expect(screen.getByLabelText('WU-1 Handler unavailable')).toBeVisible();
    expect(screen.getByLabelText('WU-1 Worker unavailable')).toBeVisible();
  });

  it('renders typed dependency meanings and opens an Agent at its lifecycle step', () => {
    const onOpenWorkUnit = vi.fn();
    const secondUnit = {
      ...unit,
      workUnitId: 'WU-2',
      workUnitScopeId: 'scope-2',
      title: 'Independent prerequisite',
    };
    const joinTarget = {
      ...unit,
      workUnitId: 'WU-3',
      workUnitScopeId: 'scope-3',
      title: 'Completion-gated work',
      dependencies: [
        { workUnitScopeId: 'scope-1', workUnitId: 'WU-1' },
        { workUnitScopeId: 'scope-2', workUnitId: 'WU-2' },
      ],
    };
    const mergedTarget = {
      ...unit,
      workUnitId: 'WU-4',
      workUnitScopeId: 'scope-4',
      title: 'Merged-result consumer',
      dependencies: [{ workUnitScopeId: 'scope-3', workUnitId: 'WU-3' }],
    };
    const plannerReport = {
      reportId: 'planner-report',
      toolName: 'record_work_slice_plan' as const,
      agentRole: 'work_slice_planner' as const,
      agentSessionRefId: 'planner-ref',
      workSlicePlanningPointId: 'plan-parallel',
      sprintPlanRevisionId: 'revision-1',
      analysisItems: [
        {
          analysisItemId: 'analysis-1',
          text: 'Join independently completed prerequisites.',
          linkedWorkUnitScopeIds: ['scope-1', 'scope-2', 'scope-3'],
        },
      ],
      workUnitScopeIds: ['scope-1', 'scope-2', 'scope-3', 'scope-4'],
      dependencies: [
        {
          dependencyId: 'independent-prerequisites',
          inputWorkUnitScopeIds: ['scope-1', 'scope-2'],
          toWorkUnitScopeId: 'scope-3',
          kind: 'merge_join' as const,
          label: 'responsive prerequisites',
          joinSemantics: 'independent_prerequisites' as const,
        },
        {
          dependencyId: 'merged-result',
          fromWorkUnitScopeId: 'scope-3',
          toWorkUnitScopeId: 'scope-4',
          kind: 'merge_join' as const,
          label: 'integrated output',
          joinSemantics: 'merged_result' as const,
        },
      ],
      provenanceId: 'recorded-plan',
    };
    render(
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={{
          ...workSlicePlanningPointGroup,
          workUnitScopeIds: ['scope-1', 'scope-2', 'scope-3', 'scope-4'],
        }}
        currentWorkState="Processing"
        workUnitRelationships={[
          {
            workUnit: unit,
            handlers: [
              {
                ...handler,
                identity: {
                  agentName: 'Rowan',
                  visualIdentity: { token: 'R', accentColor: '#945a35' },
                },
              },
            ],
            implementers: [implementer],
            handlerActivity: {
              reportId: 'handler-report',
              toolName: 'report_handler_activity',
              agentRole: 'work_unit_handler',
              agentSessionRefId: 'handler-ref',
              workUnitExecutionId: 'execution-1',
              activity: 'reviewing',
              summary: 'Reviewing the returned work.',
              lifecycleEntryId: 'lifecycle-review',
              provenanceId: 'recorded-plan',
            },
          },
          { workUnit: secondUnit, handlers: [], implementers: [] },
          { workUnit: joinTarget, handlers: [], implementers: [] },
          { workUnit: mergedTarget, handlers: [], implementers: [] },
        ]}
        plannerReport={plannerReport}
        plannerSession={session}
        onBack={vi.fn()}
        onOpenWorkUnit={onOpenWorkUnit}
      />,
    );

    const analysis = screen.getByRole('button', {
      name: 'Join independently completed prerequisites.',
    });
    fireEvent.pointerEnter(analysis);
    expect(screen.getByRole('article', { name: /Work Unit WU-1/ })).toHaveClass('is-highlighted');
    expect(screen.getByRole('article', { name: /Work Unit WU-2/ })).toHaveClass('is-highlighted');
    const independentJoin = document.querySelector(
      'svg g[data-join-semantics="independent_prerequisites"]',
    );
    expect(independentJoin).toHaveAttribute('data-input-scope-ids', 'scope-1 scope-2');
    expect(independentJoin?.querySelectorAll('[data-prerequisite-input="scope-1"]')).toHaveLength(
      1,
    );
    expect(independentJoin?.querySelectorAll('[data-prerequisite-input="scope-2"]')).toHaveLength(
      1,
    );
    expect(independentJoin?.querySelector('[data-prerequisite-input="scope-4"]')).toBeNull();
    expect(
      independentJoin?.querySelector('[data-geometry="independent-completion-gate"]'),
    ).toBeVisible();
    expect(
      independentJoin?.querySelector('.planning-point-dependency__completion-gate'),
    ).toBeVisible();
    const mergedResult = document.querySelector('svg g[data-join-semantics="merged_result"]');
    expect(mergedResult?.querySelector('[data-geometry="merged-output"]')).toHaveAttribute(
      'marker-end',
      'url(#merged-output-arrow)',
    );
    expect(mergedResult?.querySelector('.planning-point-dependency__completion-gate')).toBeNull();

    fireEvent.click(
      screen.getByRole('button', { name: /Open WU-1 Handler lifecycle at reviewing/ }),
    );
    expect(onOpenWorkUnit).toHaveBeenCalledWith('WU-1', 'lifecycle-review');
  });
});
