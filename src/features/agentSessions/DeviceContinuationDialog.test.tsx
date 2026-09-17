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
  expect(
    screen.getByText('Potential sister worktree · relationship and locks are not tracked yet.'),
  ).toBeVisible();
  expect(fixture.source.branchGraph).toHaveBeenCalledWith(
    sourceTarget.repositoryId,
    expect.any(Number),
    undefined,
  );
});
