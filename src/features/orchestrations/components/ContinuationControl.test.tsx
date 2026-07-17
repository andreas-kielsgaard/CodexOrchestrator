import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ContinuationControl } from './ContinuationControl';
import { SprintContinuationControl } from './SprintContinuationControl';

describe('continuation controls', () => {
  it('sends policy updates for both levels without requesting continuation or optimistically mutating policy', async () => {
    const epic = vi.fn(async () => ({
      status: 'unsupported' as const,
      message: 'No durable policy store.',
    }));
    const sprint = vi.fn(async () => ({
      status: 'policy_update_recorded' as const,
      message: 'Recorded policy update; canonical refresh remains authoritative.',
    }));
    const epicRequestContinuation = vi.fn();
    const sprintRequestContinuation = vi.fn();
    const epicController = {
      updatePolicy: epic,
      requestContinuation: epicRequestContinuation,
    };
    const sprintController = {
      updatePolicy: sprint,
      requestContinuation: sprintRequestContinuation,
    };
    const { rerender } = render(
      <ContinuationControl
        continuation={{
          automaticEnabled: false,
          eligible: true,
          status: 'ready_for_manual',
          policyUpdateIntent: {
            level: 'epic',
            epicId: 'o1',
            policyId: 'p1',
            automaticEnabled: false,
          },
        }}
        controller={epicController}
      />,
    );
    const epicSwitch = screen.getByRole('switch');
    fireEvent.click(epicSwitch);
    expect(epic).toHaveBeenCalledWith(expect.objectContaining({ level: 'epic', epicId: 'o1' }));
    expect(epicRequestContinuation).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('No durable policy store.'),
    );
    expect(epicSwitch).not.toBeChecked();
    rerender(
      <SprintContinuationControl
        automaticEnabled={true}
        policyUpdateIntent={{
          level: 'sprint',
          sprintId: 'e1',
          policyId: 'p2',
          automaticEnabled: true,
        }}
        controller={sprintController}
      />,
    );
    const sprintSwitch = screen.getByRole('switch');
    fireEvent.click(sprintSwitch);
    expect(sprint).toHaveBeenCalledWith(
      expect.objectContaining({ level: 'sprint', sprintId: 'e1' }),
    );
    expect(sprintRequestContinuation).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(
        'Recorded policy update; canonical refresh remains authoritative.',
      ),
    );
    expect(sprintSwitch).toBeChecked();
  });
});
