import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { repairClients } from '../workflowAuthoring/testFixtures';
import { remoteExecution, sessionTargetFixtures } from '../agentSessions/sessionTargetFixtures';
import { ExecutionConfigurationScreen } from './ExecutionConfigurationScreen';
it('discovers and saves the edited device connection and its explicit repository mapping', async () => {
  const user = userEvent.setup();
  const fixture = repairClients();
  const targets = sessionTargetFixtures();
  fixture.profiles[0] = { ...fixture.profiles[0], execution: remoteExecution };
  const local = vi.spyOn(fixture.configuration, 'loadSelectedRuntimeProfile');
  const update = vi.spyOn(fixture.configuration, 'updateCapabilityProfile');
  render(
    <ExecutionConfigurationScreen
      client={fixture.configuration}
      targetClient={targets.client}
      branchSource={targets.source}
    />,
  );
  await waitFor(() => expect(targets.client.loadRuntime).toHaveBeenCalledWith(remoteExecution));
  expect(local).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('SSH target'), { target: { value: 'workstation' } });
  await user.click(await screen.findByRole('button', { name: 'Read device capabilities' }));
  await waitFor(() =>
    expect(targets.client.loadRuntime).toHaveBeenCalledWith({
      ...remoteExecution,
      connection: { ...remoteExecution.connection, target: 'workstation' },
    }),
  );
  fireEvent.change(screen.getByLabelText('Mapped repository'), {
    target: { value: 'repository-one' },
  });
  fireEvent.change(screen.getByLabelText('Repository path on device'), {
    target: { value: '/root/projects/orchid/source' },
  });
  await user.click(screen.getByRole('button', { name: 'Save repository location' }));
  await waitFor(() =>
    expect(targets.client.saveRepositoryLocation).toHaveBeenCalledWith({
      repositoryId: 'repository-one',
      deviceId: 'remote',
      repositoryRoot: '/root/projects/orchid/source',
    }),
  );
  await user.click(screen.getByRole('button', { name: 'Save new revision' }));
  await waitFor(() =>
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        execution: {
          ...remoteExecution,
          connection: { ...remoteExecution.connection, target: 'workstation' },
        },
      }),
    ),
  );
});
it('keeps a disconnected remote profile editable without falling back to local capabilities', async () => {
  const fixture = repairClients();
  const targets = sessionTargetFixtures();
  fixture.profiles[0] = { ...fixture.profiles[0], execution: remoteExecution };
  targets.client.loadRuntime = vi.fn(async () => {
    throw new Error('Remote host unavailable');
  });
  const local = vi.spyOn(fixture.configuration, 'loadSelectedRuntimeProfile');
  render(
    <ExecutionConfigurationScreen client={fixture.configuration} targetClient={targets.client} />,
  );
  expect(await screen.findByRole('textbox', { name: 'SSH target' })).toHaveValue('orchid-remote');
  expect(await screen.findByRole('alert')).toHaveTextContent('Remote host unavailable');
  expect(local).not.toHaveBeenCalled();
});
