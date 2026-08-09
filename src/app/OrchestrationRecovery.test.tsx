import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { OrchestrationRecovery } from './OrchestrationRecovery';

describe('OrchestrationRecovery', () => {
  it('puts safe unavailable guidance before retry and keeps the raw diagnostic disclosed', () => {
    const refresh = vi.fn().mockResolvedValue(false);
    render(
      <OrchestrationRecovery
        load={{ kind: 'unavailable', reason: 'native query: access denied', refresh }}
        onPlanEpic={vi.fn()}
      />,
    );

    expect(screen.getByRole('alert')).toHaveTextContent('status is unknown until Retry succeeds');
    expect(screen.getByText('native query: access denied')).not.toBeVisible();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Plan an Epic' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(refresh).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByText('Technical details'));
    expect(screen.getByText('native query: access denied')).toBeVisible();
  });

  it('offers a typed work-unit return without promoting a new Epic plan', () => {
    const returnToWorkUnit = vi.fn();
    render(
      <OrchestrationRecovery
        load={{ kind: 'failed', message: 'transport unavailable', refresh: vi.fn() }}
        currentLocation={{
          kind: 'work_unit',
          epicId: 'epic-1',
          sprintId: 'sprint-1',
          revisionId: 'revision-1',
          workSlicePlanningPointId: 'point-1',
          workUnitId: 'unit-1',
          label: 'Verify recovery',
        }}
        onPlanEpic={vi.fn()}
        onReturnToCurrentWorkUnit={returnToWorkUnit}
      />,
    );

    expect(screen.getByRole('button', { name: 'Return to current Work Unit' })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Plan an Epic' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Return to current Work Unit' }));
    expect(returnToWorkUnit).toHaveBeenCalledOnce();
  });

  it('retains typed active drafts and their reopen action without promoting a new plan', () => {
    const openDraft = vi.fn();
    const draft = {
      epicPlanningDraftId: 'draft-1',
      agentSessionId: 'session-1',
      title: 'Existing planning draft',
      status: 'active' as const,
      createdAt: '2026-08-09T10:00:00Z',
      updatedAt: '2026-08-09T10:00:00Z',
    };
    render(
      <OrchestrationRecovery
        load={{ kind: 'unavailable', reason: 'native query unavailable', refresh: vi.fn() }}
        planningDrafts={[draft]}
        onPlanEpic={vi.fn()}
        onOpenDraft={openDraft}
      />,
    );

    expect(screen.getByRole('button', { name: /Existing planning draft/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Plan an Epic' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Existing planning draft/ }));
    expect(openDraft).toHaveBeenCalledWith(draft);
  });

  it('uses status for empty data and alert for unavailable data', () => {
    const { rerender } = render(
      <OrchestrationRecovery
        load={{ kind: 'empty', reason: 'No orchestration records.', refresh: vi.fn() }}
      />,
    );
    expect(screen.getByRole('status')).toHaveTextContent('No orchestration records are available.');
    expect(screen.queryByRole('alert')).toBeNull();

    rerender(
      <OrchestrationRecovery
        load={{ kind: 'failed', message: 'query failed', refresh: vi.fn() }}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('status is unknown until Retry succeeds');
    expect(screen.queryByRole('status')).toBeNull();
  });
});
