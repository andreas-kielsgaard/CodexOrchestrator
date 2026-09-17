import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DeviceContinuationDialog } from './DeviceContinuationDialog';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';

it('keeps the conversation branch fixed while comparing its worktrees on a selected device', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  const sourceTarget = {
    ...remoteTarget,
    capabilityProfileId: 'local-profile',
    execution: {
      deviceId: 'local',
      deviceName: 'This laptop',
      provider: 'codex' as const,
      configurationRef: 'selected',
      connection: { kind: 'local' as const },
    },
    worktreeId: 'local-worktree',
    path: 'C:\\projects\\orchid\\worktrees\\review',
  };
  render(
    <DeviceContinuationDialog
      client={fixture.client}
      source={fixture.source}
      sourceTarget={sourceTarget}
      onClose={() => {}}
    />,
  );

  await screen.findByRole('button', { name: /Remote server/ });
  await user.click(screen.getByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Select device' }));

  expect(await screen.findByText('Fixed conversation branch')).toBeVisible();
  expect(await screen.findByRole('group', { name: 'Repository branch graph' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Use branch' })).not.toBeInTheDocument();
  expect(screen.getByRole('radio', { name: /review/i })).toBeVisible();
  expect(screen.getByRole('radio', { name: /Create a new sister worktree/i })).toBeVisible();
  expect(fixture.source.branchGraph).toHaveBeenCalledWith(
    sourceTarget.repositoryId,
    expect.any(Number),
    undefined,
  );
});

it('plans and explicitly starts a session-owned device switch when the transition endpoints exist', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  const sourceTarget = {
    ...remoteTarget,
    capabilityProfileId: 'local-profile',
    execution: {
      deviceId: 'local',
      deviceName: 'This laptop',
      provider: 'codex' as const,
      configurationRef: 'selected',
      connection: { kind: 'local' as const },
    },
    worktreeId: 'local-worktree',
    path: 'C:\\projects\\orchid\\worktrees\\review',
  };
  let planned: Record<string, unknown> | undefined;
  const requested = vi.fn(async (input) => {
    planned = {
      sessionId: input.sessionId,
      sourceTarget: input.sourceTarget,
      destinationSelection: input.destinationSelection,
      phase: 'pending' as const,
      tasks: [
        { kind: 'capture_snapshot' as const, status: 'pending' as const },
        { kind: 'apply_snapshot' as const, status: 'pending' as const },
      ],
      snapshot: {
        sourceHead: sourceTarget.head,
        destinationHead: remoteTarget.head,
        totalBytes: 2 * 1024 * 1024,
      },
      transferEstimate: {
        snapshotBytes: 2 * 1024 * 1024,
        bytesPerSecond: 512 * 1024,
        estimatedSeconds: 4,
        measuredAt: '2026-09-17T12:00:00Z',
      },
    };
    return planned;
  });
  const started = vi.fn(async () => ({
    ...planned,
    phase: 'running' as const,
    tasks: [{ kind: 'capture_snapshot' as const, status: 'running' as const }],
  }));
  render(
    <DeviceContinuationDialog
      client={fixture.client}
      source={fixture.source}
      sourceTarget={sourceTarget}
      sessionId="session-one"
      profileClient={
        {
          requestTargetTransition: requested,
          loadTargetTransition: vi.fn(async () => null),
          startTargetTransition: started,
        } as never
      }
      onClose={() => {}}
    />,
  );

  await user.click(await screen.findByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Select device' }));
  await user.click(screen.getByRole('radio', { name: /review/i }));
  await user.click(screen.getByRole('button', { name: 'Review switch plan' }));

  expect(requested).toHaveBeenCalledWith(
    expect.objectContaining({ sessionId: 'session-one', sourceTarget }),
  );
  expect(await screen.findByText('Capture source worktree state')).toBeVisible();
  expect(screen.getByText(/about 4s/)).toBeVisible();

  await user.click(screen.getByRole('button', { name: 'Start switching' }));
  expect(started).toHaveBeenCalledWith('session-one');
  expect(
    await screen.findByText('Copying the source worktree state to the destination.'),
  ).toBeVisible();
});

it('keeps an existing sister fixed while still showing other branch worktrees', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  const sourceTarget = {
    ...remoteTarget,
    capabilityProfileId: 'local-profile',
    execution: {
      deviceId: 'local',
      deviceName: 'This laptop',
      provider: 'codex' as const,
      configurationRef: 'selected',
      connection: { kind: 'local' as const },
    },
    worktreeId: 'local-worktree',
    path: 'C:\\projects\\orchid\\worktrees\\review',
  };
  fixture.devices[1] = {
    ...fixture.devices[1],
    profiles: [
      {
        ...fixture.devices[1].profiles[0],
        instances: [
          {
            ...remoteTarget,
            sisterLock: {
              sisterGroupId: 'group-one',
              activeDeviceId: 'remote',
              ownerSessionId: 'session-one',
            },
            isSister: true,
          },
          {
            ...remoteTarget,
            worktreeId: 'other-worktree',
            path: '/root/projects/orchid/worktrees/other-review',
            sisterLock: {
              sisterGroupId: 'group-one',
              activeDeviceId: 'remote',
              ownerSessionId: 'session-one',
            },
            isSister: false,
          },
        ],
      },
    ],
  };
  render(
    <DeviceContinuationDialog
      client={fixture.client}
      source={fixture.source}
      sourceTarget={sourceTarget}
      sessionId="session-one"
      onClose={() => {}}
    />,
  );

  await user.click(await screen.findByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Select device' }));

  const worktreeRadios = await screen.findAllByRole('radio');
  expect(worktreeRadios[0]).toBeChecked();
  expect(screen.getByRole('radio', { name: /other-review/i })).toBeDisabled();
  expect(screen.getByRole('radio', { name: /Create a new sister worktree/i })).toBeDisabled();
});
