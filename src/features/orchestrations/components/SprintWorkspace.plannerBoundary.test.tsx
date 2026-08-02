import { render, screen } from '@testing-library/react';
import type { ProductSprintRunnerTransitionStatusV1 } from '../../../application/orchestrations';
import { WorkSlicePlannerBoundary } from './SprintWorkspace';

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

  it('labels a partial materialization without claiming settled Work Units', () => {
    render(
      <WorkSlicePlannerBoundary
        sprint={sprint(transition, [
          {
            materializationId: 'materialization-1',
            planningPointId: 'point-1',
            acceptedRevisionId: 'accepted-revision-1',
            stage: 'work_units_created',
            source: {
              status: 'available',
              sourceKind: 'application_interpretation',
              sourceReferences: ['materialization-1'],
            },
          },
        ])}
      />,
    );
    const region = screen.getByRole('region', { name: 'Work Slice Planner boundary' });
    expect(region).toHaveTextContent('Accepted revision accepted-revision-1');
    expect(region).toHaveTextContent('Work Units created; relationships not complete');
    expect(region).toHaveTextContent('No Handler activation or execution is shown.');
  });
});
