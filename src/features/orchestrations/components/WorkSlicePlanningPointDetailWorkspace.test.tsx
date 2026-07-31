import { fireEvent, render, screen } from '@testing-library/react';
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
    const relationships = screen.getByLabelText('Work Slice planning point relationships');
    expect(relationships).toHaveTextContent('Work Slice PlannerWork Slice Planner');
    expect(relationships).toHaveTextContent('WU-1Typed relationship baseline');
    expect(screen.getByLabelText('WU-1 Work Unit Handler relationship')).toHaveTextContent(
      'Typed Handler',
    );
    expect(screen.getByLabelText('WU-1 Work Unit Implementer relationship')).toHaveTextContent(
      'Typed Implementer',
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Open Work Unit WU-1: Typed relationship baseline' }),
    );
    expect(onOpenWorkUnit).toHaveBeenCalledWith('WU-1');
    expect(screen.getByLabelText('Detailed workflow unavailable')).toHaveTextContent(
      'No detailed turn sequence is recorded for this Work Slice planning point.',
    );
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

    expect(screen.getByRole('listitem', { name: /Work Unit WU-1/ })).toBeVisible();
    expect(screen.getByLabelText('WU-1 Work Unit Handler unavailable')).toBeVisible();
    expect(screen.getByLabelText('WU-1 Work Unit Implementer unavailable')).toBeVisible();
  });
});
