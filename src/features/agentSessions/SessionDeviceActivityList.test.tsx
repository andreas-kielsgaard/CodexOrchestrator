import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import type { SessionQueueEntryViewDto } from '../../application/agentSessions/sessionQueue';
import { SessionDeviceActivityList } from './SessionDeviceActivityList';

const activity = {
  entry: {
    id: 'entry',
    sessionId: 'session',
    ordinal: 0,
    state: 'pending',
    item: { kind: 'target_operation', operationId: 'operation' },
    blockedReason: null,
    createdAt: '',
    updatedAt: '',
  },
  message: null,
  targetOperation: {
    id: 'operation',
    sessionId: 'session',
    sourceTarget: null,
    needsResolution: false,
    receipt: null,
    projectedPlan: {
      calculatedAt: '2026-09-30T10:00:00Z',
      requestRevision: 0,
      stale: false,
      source: null,
      facts: { destination: null, destinationReady: null, unknown: [], blockers: [] },
      steps: [],
    },
    request: {
      intent: 'change_conversation_target' as const,
      migrationResolution: null,
      destination: {
        capabilityProfileId: 'profile',
        capabilityProfileRevision: 1,
        execution: {
          deviceId: 'remote',
          deviceName: 'Remote device',
          provider: 'codex',
          configurationRef: 'default',
          connection: { kind: 'local' },
        },
        workspace: { kind: 'auxiliary' },
      },
    },
    phase: 'staging',
    actions: [
      { kind: 'prepare_device', status: 'completed', detail: null, error: null },
      { kind: 'verify_worktree', status: 'running', detail: null, error: null },
    ],
    stagedTarget: null,
    error: null,
    generation: 0,
    createdAt: '',
    updatedAt: '',
  },
} satisfies SessionQueueEntryViewDto;

it('shows only compact active device action progress', () => {
  render(<SessionDeviceActivityList activities={[activity]} />);
  expect(screen.getByRole('complementary', { name: 'Device activity' })).toHaveTextContent(
    'Change conversation target to Remote device',
  );
  expect(screen.getByText('Verify worktree')).toBeVisible();
  expect(screen.getByLabelText('1 of 2 actions complete')).toBeVisible();
});
it('shows planned actions while waiting and supports explicit execution', () => {
  const run = vi.fn().mockResolvedValue(undefined);
  render(
    <SessionDeviceActivityList
      activities={[
        {
          ...activity,
          targetOperation: {
            ...activity.targetOperation,
            phase: 'pending',
            projectedPlan: {
              ...activity.targetOperation.projectedPlan,
              steps: activity.targetOperation.actions,
            },
          },
        },
      ]}
      onRunNow={run}
    />,
  );
  expect(screen.getByText('Waiting for the next message')).toBeVisible();
  expect(screen.getByText('Prepare device')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Run now' }));
  expect(run).toHaveBeenCalledWith('entry');
});
