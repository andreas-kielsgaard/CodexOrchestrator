import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { JevCredentialSettings } from './JevCredentialSettings';
import { createJevCredentialClient } from '../../../infrastructure/agentProviders/jev/jevCredentialClient';

describe('JEV credentials', () => {
  it('saves securely, clears the input, and removes the saved key', async () => {
    const client = {
      loadStatus: vi.fn().mockResolvedValue({ saved: false, environmentAvailable: false }),
      saveKey: vi.fn().mockResolvedValue({ saved: true, environmentAvailable: false }),
      removeKey: vi.fn().mockResolvedValue({ saved: false, environmentAvailable: false }),
    };
    const user = userEvent.setup();
    render(<JevCredentialSettings client={client} />);
    await screen.findByText('No API key configured.');
    const input = screen.getByLabelText('API key');
    expect(input).toHaveAttribute('type', 'password');
    await user.type(input, 'example-key');
    await user.click(screen.getByRole('button', { name: 'Save key' }));
    await screen.findByText('API key saved securely.');
    expect(client.saveKey).toHaveBeenCalledWith('example-key');
    expect(input).toHaveValue('');
    await user.click(screen.getByRole('button', { name: 'Remove saved key' }));
    await screen.findByText('No API key configured.');
    expect(client.removeKey).toHaveBeenCalledOnce();
  });

  it('preserves configured status when replacement fails', async () => {
    const client = {
      loadStatus: vi.fn().mockResolvedValue({ saved: true, environmentAvailable: true }),
      saveKey: vi.fn().mockRejectedValue('Credential Manager unavailable'),
      removeKey: vi.fn(),
    };
    render(<JevCredentialSettings client={client} />);
    await screen.findByText('API key saved securely.');
    await userEvent.type(screen.getByLabelText('Replace API key'), 'replacement');
    await userEvent.click(screen.getByRole('button', { name: 'Save key' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Credential Manager unavailable');
    expect(screen.getByRole('status')).toHaveTextContent('API key saved securely.');
  });

  it('uses credential-only native commands', async () => {
    const invoke = vi.fn().mockResolvedValue({ saved: true, environmentAvailable: false });
    const client = createJevCredentialClient(invoke);
    await client.loadStatus();
    await client.saveKey('example-key');
    await client.removeKey();
    expect(invoke.mock.calls).toEqual([
      ['load_jev_credential_status'],
      ['save_jev_api_key', { key: 'example-key' }],
      ['remove_jev_api_key'],
    ]);
  });
});
