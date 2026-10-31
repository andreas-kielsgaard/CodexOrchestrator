import { fireEvent, render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';
import type { RuntimeCoordinatorClient } from '../application/runtimeCoordinator/contracts';
import { App } from './App';

it('restores the setup page and non-secret draft after a top-level tab switch', async () => {
  const client = {
    overview: async () => ({
      mode: 'local',
      endpoint: null,
      coordinatorId: 'coordinator',
      error: null,
      devices: [],
      bindings: [],
      legacyDevices: [],
    }),
    connect: async () => ({
      credentialId: 'credential',
      servers: [{ id: 42, name: 'Worker', status: 'off', address: null, location: 'hel1' }],
    }),
    power: vi.fn(),
  } as unknown as RuntimeCoordinatorClient;
  render(
    <App
      {...createRecordedDevelopmentApplicationComposition({ initialSurface: 'harness-inspector' })}
      runtimeCoordinatorClient={client}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Runtime Coordinator' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Add device' }));
  fireEvent.change(screen.getByLabelText('Hetzner Cloud API token'), {
    target: { value: 'transient-secret' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Connect' }));
  await screen.findByText(/Connected · 1 servers/);
  fireEvent.change(screen.getByLabelText('Server'), { target: { value: '42' } });
  fireEvent.change(screen.getByLabelText('Display name'), { target: { value: 'My worker' } });
  fireEvent.click(screen.getByRole('button', { name: 'Harness Management' }));
  fireEvent.click(screen.getByRole('button', { name: 'Runtime Coordinator' }));
  expect(await screen.findByRole('heading', { name: 'Add Linux device' })).toBeVisible();
  expect(screen.getByLabelText('Display name')).toHaveValue('My worker');
  expect(screen.queryByLabelText('Hetzner Cloud API token')).not.toBeInTheDocument();
  expect(client.power).not.toHaveBeenCalled();
});
