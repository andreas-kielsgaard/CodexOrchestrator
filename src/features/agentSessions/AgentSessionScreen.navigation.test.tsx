import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { StandaloneAgentSessionScreen } from './AgentSessionScreen';
import { repairSessionClients } from './profileTestFixtures';
import { navigationData, recordedNavigation } from './navigationTestFixtures';
it('keeps a folder draft through refresh and sends the current folder through the profiled path', async () => {
  const fixture = repairSessionClients(false);
  const start = vi.spyOn(fixture.profiles, 'startDirectUserSession');
  const navigation = recordedNavigation({
    ...navigationData(),
    summaries: [],
    organization: [],
    owners: [],
  });
  render(
    <StandaloneAgentSessionScreen
      client={fixture.sessions}
      profileClient={fixture.profiles}
      navigationClient={navigation}
    />,
  );
  fireEvent.click(await screen.findByRole('button', { name: 'New session in Alpha' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Repo draft' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Refresh' })).not.toBeDisabled());
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue('Repo draft');
  fireEvent.click(screen.getByRole('button', { name: 'New session in Feature build' }));
  expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue('');
  fireEvent.change(screen.getByRole('textbox', { name: 'Message' }), {
    target: { value: 'Instance notes' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Send' }));
  await waitFor(() =>
    expect(start).toHaveBeenCalledWith(
      expect.objectContaining({
        submittedText: 'Instance notes',
        folderTarget: { kind: 'workflow_instance', instanceId: 'flow-a' },
      }),
    ),
  );
});
it('does not reset a conversation draft on pin and move', async () => {
  const fixture = repairSessionClients();
  const navigation = recordedNavigation();
  render(
    <StandaloneAgentSessionScreen
      client={fixture.sessions}
      profileClient={fixture.profiles}
      navigationClient={navigation}
    />,
  );
  const message = await screen.findByRole('textbox', { name: 'Message' });
  fireEvent.change(message, { target: { value: 'Keep my draft' } });
  fireEvent.contextMenu(await screen.findByRole('treeitem', { name: 'Session 1' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Pin' }));
  await waitFor(() =>
    expect(screen.getAllByRole('treeitem', { name: 'Session 1' })).toHaveLength(2),
  );
  expect(message).toHaveValue('Keep my draft');
  fireEvent.contextMenu(screen.getAllByRole('treeitem', { name: 'Session 1' })[0]);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Move to…' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Empty repo' }));
  await waitFor(() => expect(message).toHaveValue('Keep my draft'));
});
