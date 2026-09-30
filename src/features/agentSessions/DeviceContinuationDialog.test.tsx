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
  expect(screen.getByRole('radio', { name: /Create a new worktree/i })).toBeVisible();
  expect(fixture.source.branchGraph).toHaveBeenCalledWith(
    sourceTarget.repositoryId,
    expect.any(Number),
    undefined,
  );
});

it('queues a selected device and worktree as one target operation', async () => {
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
  const queued = vi.fn(async () => {});
  render(
    <DeviceContinuationDialog
      client={fixture.client}
      source={fixture.source}
      sourceTarget={sourceTarget}
      sessionId="session-one"
      onQueueSelection={queued}
      onClose={() => {}}
    />,
  );

  await user.click(await screen.findByRole('button', { name: /Remote server/ }));
  await user.click(screen.getByRole('button', { name: 'Select device' }));
  await user.click(screen.getByRole('radio', { name: /review/i }));
  await user.click(screen.getByRole('button', { name: 'Queue device switch' }));

  expect(queued).toHaveBeenCalledWith(
    expect.objectContaining({
      intent: 'change_conversation_target',
      destination: expect.objectContaining({
        execution: remoteTarget.execution,
        workspace: expect.objectContaining({ kind: 'existing', target: remoteTarget }),
      }),
    }),
  );
});

it('allows retargeting to unlinked worktrees and explicit migration choices', async () => {
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
  expect(screen.getByRole('radio', { name: /other-review/i })).toBeEnabled();
  expect(screen.getByRole('radio', { name: /Create a new worktree/i })).toBeEnabled();
});
