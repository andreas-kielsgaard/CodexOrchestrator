import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { EpicPauseRestartController, EpicPauseRestartQuery, EpicPauseRestartOutcome } from '../../../application/orchestrations';
import { EpicPauseRestartControl } from './EpicPauseRestartControl';

const outcome = (kind: EpicPauseRestartOutcome['kind'], status: EpicPauseRestartOutcome['status'], targetCount = 2, launchedCount = 0): EpicPauseRestartOutcome => ({
  actionId: `action-${status}`,
  kind,
  status,
  targetCount,
  launchedCount,
});
const query = (pause: EpicPauseRestartQuery['pause'], restart: EpicPauseRestartQuery['restart'] = { availability: 'unavailable', reason: 'No interrupted orchestration conversation is eligible for Restart.' }): EpicPauseRestartQuery => ({ epicId: 'epic-1', pause, restart });

function controllerFor(initial: EpicPauseRestartQuery, afterRequest = initial) {
  let current = initial;
  return {
    controller: {
      load: vi.fn(async () => current),
      requestPause: vi.fn(async () => {
        current = afterRequest;
        return afterRequest.pause.current ?? outcome('pause', 'pending', 0, 0);
      }),
      requestRestart: vi.fn(async () => {
        current = afterRequest;
        return afterRequest.restart.current ?? outcome('restart', 'pending', 0, 0);
      }),
    } satisfies EpicPauseRestartController,
  };
}

describe('Epic Pause/Restart control', () => {
  it('loads authoritative state, preserves factual reasons, and reloads after Pause', async () => {
    const initial = query({ availability: 'available', reason: 'Pause dispatch is available.' });
    const afterPause = query({ availability: 'busy', reason: 'A durable Pause request is still reconciling.', current: outcome('pause', 'pending', 2, 0) });
    const { controller } = controllerFor(initial, afterPause);
    render(<EpicPauseRestartControl epicId="epic-1" controller={controller} />);

    await waitFor(() => expect(screen.getByText(/Pause dispatch is available/)).toBeVisible());
    expect(screen.getByRole('button', { name: 'Pause' })).toBeEnabled();
    expect(screen.getByText(/Pause dispatch is available/)).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(controller.requestPause).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(controller.load).toHaveBeenCalledTimes(2));
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByText(/Pause pending: 0 of 2 dispatches launch-accepted/)).toBeVisible();
    expect(screen.getAllByText(/Provider receipt, compliance, and progress are not observed/).length).toBeGreaterThan(0);
  });

  it.each([
    ['zero-target unavailable', query({ availability: 'unavailable', reason: 'No working orchestration conversation is eligible for Pause.', current: outcome('pause', 'completed', 0, 0) })],
    ['partial', query({ availability: 'unavailable', reason: 'Pause completed with attention.', current: outcome('pause', 'partial', 3, 2) })],
    ['attention', query({ availability: 'unavailable', reason: 'Pause requires attention.', current: outcome('pause', 'attention', 2, 1) })],
    ['completed', query({ availability: 'unavailable', reason: 'Pause completed.', current: outcome('pause', 'completed', 2, 2) })],
  ])('renders the factual %s state and disables unavailable controls', async (_label, state) => {
    const { controller } = controllerFor(state);
    render(<EpicPauseRestartControl epicId="epic-1" controller={controller} />);
    await waitFor(() => expect(screen.getByText(new RegExp(state.pause.current!.status))).toBeVisible());
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Restart' })).toBeDisabled();
  });

  it('loads authoritative Restart state, guards duplicate submission, reloads, and renders its persisted outcome', async () => {
    const initial = query(
      { availability: 'unavailable', reason: 'No working orchestration conversation is eligible for Pause.' },
      { availability: 'available', reason: 'Restart dispatch is available.' },
    );
    const afterRestart = query(
      { availability: 'unavailable', reason: 'Pause is not available while the Epic is interrupted.' },
      { availability: 'busy', reason: 'A durable Restart request is still reconciling.', current: outcome('restart', 'completed', 2, 2) },
    );
    const { controller } = controllerFor(initial, afterRestart);
    render(<EpicPauseRestartControl epicId="epic-1" controller={controller} />);

    await waitFor(() => expect(screen.getByText(/Restart dispatch is available/)).toBeVisible());
    expect(screen.getByRole('button', { name: 'Restart' })).toBeEnabled();
    fireEvent.click(screen.getByRole('button', { name: 'Restart' }));
    fireEvent.click(screen.getByRole('button', { name: 'Restart' }));
    expect(controller.requestRestart).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(controller.load).toHaveBeenCalledTimes(2));
    expect(screen.getByRole('button', { name: 'Restart' })).toBeDisabled();
    expect(screen.getByText(/A durable Restart request is still reconciling/)).toBeVisible();
    expect(screen.getByText(/Restart completed: 2 of 2 dispatches launch-accepted/)).toBeVisible();
    expect(screen.getAllByText(/Provider receipt, compliance, and progress are not observed/).length).toBeGreaterThan(0);
  });
});
