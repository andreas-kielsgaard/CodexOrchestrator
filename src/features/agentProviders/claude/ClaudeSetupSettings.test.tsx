import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ProviderSetupDto } from '../../../application/agentProviders';
import type { ClaudeSetupClient } from '../../../infrastructure/agentProviders/claude/claudeSetupClient';
import { createClaudeSetupClient } from '../../../infrastructure/agentProviders/claude/claudeSetupClient';
import { ClaudeSetupSettings } from './ClaudeSetupSettings';

const setup = (overrides: Partial<ProviderSetupDto> = {}): ProviderSetupDto => ({
  deviceId: 'local',
  provider: 'claude',
  configurationId: 'claude-default',
  folder: '/home/me/.claude',
  executable: 'claude',
  state: 'ready',
  detail: null,
  selected: true,
  ...overrides,
});

describe('ClaudeSetupSettings', () => {
  it('lists setups with their sign-in state and adds a folder', async () => {
    let setups: ProviderSetupDto[] = [setup({ state: 'needs_login', detail: 'Sign in first.' })];
    const client: ClaudeSetupClient = {
      listSetups: vi.fn(async () => setups),
      addSetup: vi.fn(async (folder: string) => {
        setups = [...setups, setup({ configurationId: 'claude-work', folder, selected: false })];
      }),
      removeSetup: vi.fn(async () => undefined),
    };
    render(<ClaudeSetupSettings client={client} />);
    expect(await screen.findByText('Sign in first.')).toBeTruthy();
    expect(screen.getByText(/Needs sign-in/)).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Configuration folder'), {
      target: { value: '/work/claude' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add setup' }));
    await waitFor(() => expect(client.addSetup).toHaveBeenCalledWith('/work/claude', ''));
    expect(await screen.findByRole('heading', { name: '/work/claude' })).toBeTruthy();
    fireEvent.click(screen.getAllByRole('button', { name: 'Remove' })[1]);
    await waitFor(() => expect(client.removeSetup).toHaveBeenCalledWith('claude-work'));
  });

  it('shows only Claude setups and sends the add and remove commands', async () => {
    const invoke = vi.fn(async (command: string) =>
      command === 'list_provider_setups' ? [setup(), setup({ provider: 'codex' })] : undefined,
    ) as unknown as Parameters<typeof createClaudeSetupClient>[0];
    const client = createClaudeSetupClient(invoke);
    expect((await client.listSetups()).map((entry) => entry.provider)).toEqual(['claude']);
    await client.addSetup('/work/claude', ' ');
    await client.removeSetup('claude-work');
    expect(invoke).toHaveBeenCalledWith('add_claude_setup', {
      input: { folder: '/work/claude', executable: null },
    });
    expect(invoke).toHaveBeenCalledWith('remove_claude_setup', { setupId: 'claude-work' });
  });
});
