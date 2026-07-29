import { render, screen } from '@testing-library/react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { SprintAgentSessionPresentation } from '../orchestrationModel';
import { SprintPlannerActivityDetailWorkspace } from './SprintPlannerActivityDetailWorkspace';

const source = {
  status: 'available' as const,
  sourceKind: 'orchestration_event' as const,
  sourceReferences: ['recorded-plan'],
};

const plannerActivityGroup: SprintWorkspacePresentationV1['revisionViews'][number]['plannerActivityGroups'][number] =
  {
    sprintPlannerActivityId: 'plan-parallel',
    title: 'Parallel planning',
    purpose: 'Coordinate two planning conversations.',
    source,
    membershipSource: source,
    workUnitScopeIds: [],
  };

const sessions: readonly SprintAgentSessionPresentation[] = [
  { sessionId: 'planner-primary', title: 'Primary planner' },
  { sessionId: 'planner-reviewer', title: 'Planning reviewer' },
];

describe('SprintPlannerActivityDetailWorkspace', () => {
  it('uses the reusable horizontal split when two Agent Sessions are shown', () => {
    render(
      <SprintPlannerActivityDetailWorkspace
        plannerActivityGroup={plannerActivityGroup}
        sessions={sessions}
        onBack={vi.fn()}
      />,
    );

    expect(
      screen.getByRole('separator', {
        name: 'Resize Primary planner conversation and Planning reviewer conversation',
      }),
    ).toHaveAttribute('aria-orientation', 'vertical');
    expect(screen.getByLabelText('Primary planner Agent Session')).toBeVisible();
    expect(screen.getByLabelText('Planning reviewer Agent Session')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Collapse Agent Session' })).toBeNull();
  });
});
