import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ForkConversationDialog } from './ForkConversationDialog';
import { repairSessionClients } from './profileTestFixtures';
import { remoteTarget, sessionTargetFixtures } from './sessionTargetFixtures';
import { sessionDetails } from './testFixtures';

it('requires an explicit destination and reuses the fork identity after a lost reply', async () => {
  const user = userEvent.setup();
  const { sessions } = repairSessionClients();
  const { client } = sessionTargetFixtures();
  const created = vi.fn();
  const close = vi.fn();
  const fork = vi
    .fn()
    .mockRejectedValueOnce(new Error('Reply lost'))
    .mockResolvedValue({ ...sessionDetails().session, id: 'forked-session' });
  sessions.forkConversation = fork;
  render(
    <ForkConversationDialog
      sourceId="offline-source"
      sessions={sessions}
      targets={client}
      onCreated={created}
      onClose={close}
    />,
  );
  expect(screen.getByRole('button', { name: 'Create fork' })).toBeDisabled();
  await screen.findByRole('option', { name: 'Remote server' });
  await user.selectOptions(screen.getByLabelText('Execution device'), 'remote');
  await screen.findByRole('option', { name: /\/root\/projects\/orchid\/worktrees\/review/ });
  await user.selectOptions(screen.getByLabelText('New worktree target'), '0');
  await user.click(screen.getByRole('button', { name: 'Create fork' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Reply lost');
  expect(created).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Create fork' }));
  expect(fork).toHaveBeenCalledTimes(2);
  expect(fork.mock.calls[1]).toEqual(fork.mock.calls[0]);
  expect(fork.mock.calls[0]).toEqual(['offline-source', expect.any(String), remoteTarget]);
  expect(created).toHaveBeenCalledWith('forked-session');
  expect(close).toHaveBeenCalledOnce();
});
