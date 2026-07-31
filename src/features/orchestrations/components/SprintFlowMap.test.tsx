import { fireEvent, render, screen, within } from '@testing-library/react';
import { useState } from 'react';
import { composeProductOrchestrationReadModels } from '../../../application/orchestrations';
import { presentProductOrchestrations } from '../../../app/orchestrationPresentation';
import { recordedPresentationAdjunct } from '../../../dev/orchestrationSection/recordedPresentationAdjunct';
import { recordedProductReadCompositionInput } from '../../../dev/orchestrationSection/recordedProductReadCompositionInput';
import {
  projectSprintConnectorRoutes,
  projectSprintFlowConnectors,
  projectSprintFlowLayout,
} from '../sprintFlowLayout';
import { SprintFlowMap } from './SprintFlowMap';

function StatefulSprintFlowMap({
  onOpenWorkSlicePlanningPointGroup,
  onOpenWorkUnit,
}: {
  readonly onOpenWorkSlicePlanningPointGroup?: (
    workSlicePlanningPointId: string,
    opener: HTMLButtonElement,
  ) => void;
  readonly onOpenWorkUnit?: (workUnitId: string, opener: HTMLButtonElement) => void;
}) {
  const [selectedRevisionId, setSelectedRevisionId] = useState(
    recordedWorkspace.selectedSprintPlanRevisionId,
  );
  return (
    <SprintFlowMap
      workspace={recordedWorkspace}
      selectedRevisionId={selectedRevisionId}
      onSelectedRevisionChange={setSelectedRevisionId}
      onOpenWorkSlicePlanningPointGroup={onOpenWorkSlicePlanningPointGroup}
      onOpenWorkUnit={onOpenWorkUnit}
    />
  );
}

describe('SprintFlowMap', () => {
  it('renders Work Slice planning-point grouping with accepted Plan copy and actionable Work Units', () => {
    const onOpenWorkSlicePlanningPointGroup = vi.fn();
    const onOpenWorkUnit = vi.fn();
    render(
      <StatefulSprintFlowMap
        onOpenWorkSlicePlanningPointGroup={onOpenWorkSlicePlanningPointGroup}
        onOpenWorkUnit={onOpenWorkUnit}
      />,
    );

    const plan = screen.getByRole('region', { name: 'Plan: Integrated detail surfaces' });
    expect(
      within(plan).getByRole('button', { name: 'Open Plan: Integrated detail surfaces' }),
    ).toBeVisible();
    const workUnit = screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ });
    const planButton = within(plan).getByRole('button', {
      name: 'Open Plan: Integrated detail surfaces',
    });
    expect(planButton).toHaveClass('sprint-plan-region__open');
    fireEvent.click(planButton);
    expect(onOpenWorkSlicePlanningPointGroup).toHaveBeenCalledTimes(1);
    expect(onOpenWorkSlicePlanningPointGroup).toHaveBeenCalledWith(
      'planner-r4-integration',
      expect.any(HTMLButtonElement),
    );
    onOpenWorkSlicePlanningPointGroup.mockClear();
    fireEvent.click(workUnit);
    expect(onOpenWorkSlicePlanningPointGroup).not.toHaveBeenCalled();
    expect(onOpenWorkUnit).toHaveBeenCalledWith('WU-ECS2E', expect.any(HTMLButtonElement));
    expect(screen.queryByRole('button', { name: /^View WU-/ })).toBeNull();
    expect(document.querySelector('.lucide-flag')).toBeNull();
    expect(screen.getByLabelText('G2-R4 user review for WU-ECS3')).toBeVisible();
  });

  it('keeps the revision selector outside the scrolling viewport and switches honest history', () => {
    render(<StatefulSprintFlowMap />);
    const select = screen.getByRole('combobox', { name: 'Plan revision' });
    expect(select).toHaveValue('ECS-R4');
    expect(select.closest('.sprint-flow__overlay')).not.toBeNull();
    expect(select.closest('.sprint-flow__viewport')).toBeNull();
    expect(screen.getByRole('option', { name: 'ECS-R4 - Current' })).toBeVisible();

    fireEvent.change(select, { target: { value: 'ECS-R1' } });
    expect(screen.getByLabelText('ECS-R1 Sprint and Work Unit planning')).toBeVisible();
    expect(screen.getByRole('button', { name: /Open Work Unit WU-ECS2:/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: /WU-ECS2E/ })).toBeNull();

    fireEvent.change(select, { target: { value: 'ECS-R4' } });
    expect(screen.queryByText('User visual evaluation required')).toBeNull();
  });

  it('renders canonical dependency connectors without disposable demo nodes', () => {
    const view = recordedWorkspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4',
    )!;
    expect(view.workUnits.map(({ workUnitId }) => workUnitId)).toEqual(
      expect.arrayContaining(['WU-ECS2C', 'WU-ECS2E', 'WU-ECS2D']),
    );
    expect(projectSprintFlowConnectors(view)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ from: 'WU-ECS2C', to: 'WU-ECS2E' }),
        expect.objectContaining({ from: 'WU-ECS2E', to: 'WU-ECS2D' }),
      ]),
    );
    render(<StatefulSprintFlowMap />);
    expect(screen.getByRole('button', { name: /WU-ECS2E.*Completed/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: /WU-DEMO-/ })).toBeNull();
  });

  it('supports keyboard activation through native Plan, Work Unit, revision, and plan-change controls', () => {
    render(<StatefulSprintFlowMap />);
    const plan = screen.getByRole('button', { name: 'Open Plan: Integrated detail surfaces' });
    const workUnit = screen.getByRole('button', { name: /Open Work Unit WU-ECS2E/ });
    for (const control of [plan, workUnit, screen.getByRole('combobox')]) {
      control.focus();
      expect(control).toHaveFocus();
    }
  });

  it('uses straight lanes and orthogonal routes scoped to Plan clips or map gutters', () => {
    const view = recordedWorkspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === 'ECS-R4',
    )!;
    const layout = projectSprintFlowLayout(view);
    const routes = projectSprintConnectorRoutes(view, layout);
    expect(routes.every(({ path }) => !/\sL\s/.test(path))).toBe(true);
    expect(routes.every(({ path }) => path.includes('H'))).toBe(true);
    const foundationIds = ['WU-ECS1', 'WU-ECS2A', 'WU-ECS2B', 'WU-ECS2C'];
    expect(layout.positions.filter(({ id }) => foundationIds.includes(id))).toHaveLength(4);

    render(<StatefulSprintFlowMap />);
    const paths = [...document.querySelectorAll<SVGPathElement>('path[data-connector]')];
    expect(paths.every((path) => !/\sL\s/.test(path.getAttribute('d') ?? ''))).toBe(true);
    expect(
      paths
        .filter((path) => path.dataset.connectorScope !== 'map-gutter')
        .every((path) => path.closest('.sprint-plan-region__connectors')),
    ).toBe(true);
    expect(
      paths
        .filter((path) => path.dataset.connectorScope === 'map-gutter')
        .every(
          (path) => path.closest('.sprint-map-canvas') && !path.closest('.sprint-plan-region'),
        ),
    ).toBe(true);
  });
});

const recordedWorkspace = presentProductOrchestrations(
  composeProductOrchestrationReadModels(recordedProductReadCompositionInput),
  recordedPresentationAdjunct,
).epics[0].plan.items[2].workspace!;
