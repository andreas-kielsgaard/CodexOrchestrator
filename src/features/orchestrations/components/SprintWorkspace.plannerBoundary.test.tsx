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
      'No Planner proposal or result, Work Unit, Handler, Implementer, settlement, or later planning point is recorded here.',
    );
    expect(screen.getByRole('region', { name: 'Work Slice Planner boundary' })).toHaveTextContent(
      'Planner request',
    );
    expect(screen.getByRole('region', { name: 'Work Slice Planner boundary' })).toHaveTextContent(
      'Planner authorization',
    );
  });
});
