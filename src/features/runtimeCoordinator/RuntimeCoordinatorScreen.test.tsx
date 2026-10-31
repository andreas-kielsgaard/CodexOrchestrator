import { useState } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type {
  CoordinatorDevice,
  CoordinatorLocation,
  CoordinatorOverview,
  DeviceSetupDraft,
  RuntimeCoordinatorClient,
} from '../../application/runtimeCoordinator/contracts';
import { RuntimeCoordinatorScreen } from './RuntimeCoordinatorScreen';

const device: CoordinatorDevice = {
  id: 'device-1',
  displayName: 'Worker',
  server: { id: 42, name: 'linux-worker', status: 'off', address: '192.0.2.42', location: 'hel1' },
  observedAt: '2026-10-04T09:00:00Z',
  credentialConfigured: true,
  operation: null,
  observationError: null,
};
const overview: CoordinatorOverview = {
  mode: 'local',
  endpoint: null,
  coordinatorId: 'coordinator-1',
  error: null,
  devices: [],
  bindings: [],
  legacyDevices: [],
};
function fixture(initial: CoordinatorOverview = overview) {
  let current = initial;
  const client: RuntimeCoordinatorClient = {
    overview: vi.fn(async () => current),
    dockerSettings: vi.fn(async () => ({
      ...current,
      mode: 'external' as const,
      endpoint: 'http://127.0.0.1:41800',
    })),
    settings: vi.fn(async () => current),
    connect: vi.fn(async () => ({ credentialId: 'credential-ref', servers: [device.server] })),
    register: vi.fn(async (input) => {
      const next = { ...device, displayName: input.displayName };
      current = { ...current, devices: [next] };
      return next;
    }),
    refresh: vi.fn(async () => current.devices[0] ?? device),
    rename: vi.fn(async (_id, name) => ({ ...device, displayName: name })),
    power: vi.fn(async (id, input) => ({
      id: input.operationId,
      deviceId: id,
      command: input.command,
      status: 'running',
      providerActionId: 99,
      requestedAt: '2026-10-04T09:00:00Z',
      detail: null,
    })),
  };
  function Harness() {
    const [location, navigate] = useState<CoordinatorLocation>({ view: 'list' });
    const [draft, changeDraft] = useState<DeviceSetupDraft | null>(null);
    return (
      <RuntimeCoordinatorScreen
        client={client}
        location={location}
        navigate={navigate}
        draft={draft}
        changeDraft={changeDraft}
      />
    );
  }
  return { client, Harness };
}

describe('Runtime Coordinator', () => {
  it('connects the laptop Docker deployment without a frontend token or Hetzner request', async () => {
    const { client, Harness } = fixture();
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(await screen.findByRole('button', { name: 'Configure coordinator' }));
    await user.click(screen.getByRole('radio', { name: 'External coordinator' }));
    await user.click(screen.getByRole('button', { name: 'Connect to laptop Docker' }));
    expect(await screen.findByText('http://127.0.0.1:41800 · Connected')).toBeVisible();
    expect(client.dockerSettings).toHaveBeenCalledOnce();
    expect(client.settings).not.toHaveBeenCalled();
    expect(client.connect).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Add device' })).toBeEnabled();
  });

  it('adds an existing server without powering it, clears the token, and opens detail', async () => {
    const { client, Harness } = fixture();
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(await screen.findByRole('button', { name: 'Add device' }));
    await user.type(screen.getByLabelText('Hetzner Cloud API token'), 'transient-secret');
    await user.click(screen.getByRole('button', { name: 'Connect' }));
    await screen.findByText(/Connected · 1 servers/);
    expect(screen.queryByLabelText('Hetzner Cloud API token')).not.toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Server'), '42');
    await user.clear(screen.getByLabelText('Display name'));
    await user.type(screen.getByLabelText('Display name'), 'My worker');
    await user.click(screen.getByRole('button', { name: 'Add device' }));
    expect(await screen.findByRole('heading', { name: 'My worker' })).toBeVisible();
    expect(client.register).toHaveBeenCalledWith({
      credentialId: 'credential-ref',
      serverId: 42,
      displayName: 'My worker',
      executionDeviceId: null,
    });
    expect(client.power).not.toHaveBeenCalled();
    expect(screen.getByText('Not configured')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Start' }));
    await waitFor(() =>
      expect(client.power).toHaveBeenCalledWith('device-1', {
        operationId: expect.any(String),
        command: 'start',
      }),
    );
  });
  it('keeps a failed external connection selected and allows explicit reconfiguration', async () => {
    const { Harness } = fixture({
      ...overview,
      mode: 'external',
      endpoint: 'https://coordinator.example',
      coordinatorId: null,
      error: 'Coordinator is unavailable',
    });
    render(<Harness />);
    expect(await screen.findByText(/https:\/\/coordinator.example · Disconnected/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Add device' })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'Configure coordinator' }));
    expect(screen.getByLabelText('External coordinator')).toBeChecked();
    expect(screen.getByLabelText('Coordinator URL')).toHaveValue('https://coordinator.example');
  });

  it('requires a fresh observation after a lost submission acknowledgement', async () => {
    const { client, Harness } = fixture({ ...overview, devices: [device] });
    vi.mocked(client.power).mockRejectedValue(new Error('Coordinator did not reply'));
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(await screen.findByRole('button', { name: /Worker.*Hetzner Linux/s }));
    await screen.findByRole('heading', { name: 'Worker' });
    await user.click(screen.getByRole('button', { name: 'Start' }));
    expect(await screen.findByText(/Refresh before issuing another power request/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Start' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Start' })).toBeEnabled());
    expect(client.power).toHaveBeenCalledTimes(1);
  });
});
