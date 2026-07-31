import { render, screen } from '@testing-library/react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { SprintAgentSessionPresentation } from '../orchestrationModel';
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
    workUnitScopeIds: [],
  };

const session: SprintAgentSessionPresentation = {
  sessionId: 'planner-primary',
  title: 'Work Slice Planner',
};

describe('WorkSlicePlanningPointDetailWorkspace', () => {
  it('shows one Work Slice Planner Session for the temporal planning point', () => {
    render(
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={workSlicePlanningPointGroup}
        currentWorkState="Processing"
        session={session}
        onBack={vi.fn()}
      />,
    );

    expect(screen.getByLabelText('Plan controls')).toHaveTextContent('Current workProcessing');
    expect(screen.getByLabelText('Work Slice Planner Agent Session')).toBeVisible();
    expect(screen.queryAllByLabelText(/Agent Session$/)).toHaveLength(1);
    expect(screen.queryByRole('button', { name: 'Collapse Agent Session' })).toBeNull();
  });
});
