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

describe('Work Slice Planner boundary disclosure', () => {
  it('is absent before a durable Planner request', () => {
    render(<WorkSlicePlannerBoundary transition={undefined} />);
    expect(screen.queryByRole('region', { name: 'Work Slice Planner boundary' })).toBeNull();
  });

  it('is present after a durable Planner request and states the downstream stop', () => {
    render(<WorkSlicePlannerBoundary transition={transition} />);
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
        transition={{
          ...transition,
          workSliceProposalSubmittedAt: '2026-08-02T00:00:02Z',
          workSliceProposalValidationResult: 'valid',
          workSliceRefinementRequestedAt: '2026-08-02T00:00:03Z',
          workSliceSemanticCompletedAt: undefined,
          workSliceTerminalLifecycleObservedAt: undefined,
          workSliceApplicationAcceptedAt: undefined,
          workSliceMaterializationReadyAt: undefined,
        }}
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
});
