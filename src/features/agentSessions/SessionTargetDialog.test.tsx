import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SessionTargetDialog } from './SessionTargetDialog';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';

it('requires repository and branch before listing grouped devices, then returns the profile and worktree', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  const onSelect = vi.fn();
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={null}
      onClose={() => {}}
      onSelect={onSelect}
    />,
  );
  await screen.findByRole('option', { name: /Codex Orchestrator/ });
  expect(fixture.source.branchGraph).not.toHaveBeenCalled();
  expect(fixture.client.listTargets).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('Target repository'), {
    target: { value: 'repository-one' },
  });
  await user.click(await screen.findByRole('button', { name: /^codex\/durable-review / }));
  expect(await screen.findByText('No existing worktree for this branch.')).toBeVisible();
  expect(
    within(screen.getByRole('article', { name: 'This laptop' })).getByText('Laptop Codex · Codex'),
  ).toBeVisible();
  await user.click(screen.getByRole('button', { name: new RegExp(remoteTarget.path) }));
  await user.click(screen.getByRole('button', { name: 'Use worktree' }));
  expect(onSelect).toHaveBeenCalledWith(remoteTarget);
  expect(fixture.client.listTargets).toHaveBeenCalledWith('repository-one', remoteTarget.branchRef);
});
it('uses the shared graph inside the same dialog and distinguishes unavailable devices from empty ones', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  fixture.devices[1] = {
    ...fixture.devices[1],
    profiles: [
      { ...fixture.devices[1].profiles[0], instances: [], error: 'SSH connection refused' },
    ],
  };
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={null}
      onClose={() => {}}
      onSelect={() => {}}
    />,
  );
  await screen.findByRole('option', { name: /Codex Orchestrator/ });
  fireEvent.change(screen.getByLabelText('Target repository'), {
    target: { value: 'repository-one' },
  });
  await user.click(await screen.findByRole('button', { name: 'Select branch…' }));
  expect(screen.getAllByRole('dialog')).toHaveLength(1);
  expect(await screen.findByRole('group', { name: 'Repository branch graph' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'codex/durable-review' }));
  await user.click(screen.getByRole('button', { name: 'Use branch' }));
  expect(await screen.findByText('Unavailable: SSH connection refused')).toBeVisible();
  expect(screen.getByText('No existing worktree for this branch.')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Use worktree' })).toBeDisabled();
});
it('displays a worktree with an unknown HEAD without treating it as an absent instance', async () => {
  const fixture = sessionTargetFixtures();
  fixture.devices[1] = {
    ...fixture.devices[1],
    profiles: [
      {
        ...fixture.devices[1].profiles[0],
        instances: [{ ...fixture.devices[1].profiles[0].instances[0], head: null }],
      },
    ],
  };
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={remoteTarget}
      onClose={() => {}}
      onSelect={() => {}}
    />,
  );
  await waitFor(() => expect(screen.getByText('HEAD unknown')).toBeVisible());
});

it('keeps a reopened selection unavailable when its current device listing fails', async () => {
  const user = userEvent.setup();
  const fixture = sessionTargetFixtures();
  fixture.devices[1] = {
    ...fixture.devices[1],
    profiles: [
      { ...fixture.devices[1].profiles[0], error: 'SSH connection refused', instances: [] },
    ],
  };
  const onSelect = vi.fn();
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={remoteTarget}
      onClose={() => {}}
      onSelect={onSelect}
    />,
  );
  expect(await screen.findByText('Unavailable: SSH connection refused')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Use worktree' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Use worktree' }));
  expect(onSelect).not.toHaveBeenCalled();
});
it('explains where to register a repository when the catalog is empty', async () => {
  const fixture = sessionTargetFixtures();
  fixture.source.listRepositories = vi.fn(async () => []);
  render(
    <SessionTargetDialog
      client={fixture.client}
      source={fixture.source}
      selected={null}
      onClose={() => {}}
      onSelect={() => {}}
    />,
  );
  expect(
    await screen.findByText(/No repositories are registered. Add a repository in Worktree Review/),
  ).toBeVisible();
  expect(fixture.source.branchGraph).not.toHaveBeenCalled();
});
