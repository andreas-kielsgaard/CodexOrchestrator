import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NativeProfileSettings } from './NativeProfileSettings';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';

const profiles = [
  {
    id: 'p1',
    homePath: '\\\\?\\C:\\Users\\user\\.codex',
    ownership: 'registered_existing',
    lifecycle: 'active',
    selected: true,
    readiness: { authentication: 'unknown' },
  },
  {
    id: 'p2',
    homePath: 'C:/Orchid/profile-two',
    ownership: 'application_dedicated',
    lifecycle: 'active',
    selected: false,
    readiness: { authentication: 'unauthenticated' },
  },
] as unknown as readonly NativeProfile[];

function client(overrides: Partial<NativeProfileClient> = {}): NativeProfileClient {
  const query = async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles });
  return {
    load: query,
    discoverHomes: async () => [{ homePath: 'C:/Users/user/.codex-review', source: 'sibling', registeredProfileId: null, selected: false }],
    registerExisting: query,
    createDedicated: query,
    select: query,
    requestLogin: query,
    refreshReadiness: query,
    openInExplorer: async () => {},
    loadHarnessTools: async () => ({ entries: [], limitations: [] }),
    loadSkills: async () => ({ skills: [], limitations: [] }),
    ...overrides,
  };
}

describe('NativeProfileSettings', () => {
  it('discovers skills from the selected registered Codex profile without a session', async () => {
    const user = userEvent.setup();
    const loadSkills = vi.fn(async (profileId: string) => ({
      skills: [{ name: `skill-${profileId}`, description: 'A discovered skill', path: `C:/skills/${profileId}/SKILL.md`, scope: 'user', enabled: true }],
      limitations: [],
    }));
    render(<NativeProfileSettings client={client({ loadSkills })}/>);
    expect(await screen.findByText('skill-p1')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /C:\/Orchid\/profile-two/ }));
    expect(await screen.findByText('skill-p2')).toBeInTheDocument();
    expect(screen.queryByText('skill-p1')).not.toBeInTheDocument();
    expect(loadSkills).toHaveBeenCalledWith('p1');
    expect(loadSkills).toHaveBeenCalledWith('p2');
  });
  it('uses a profile list as primary navigation and keeps the default indicator there', async () => {
    const user = userEvent.setup();
    render(<NativeProfileSettings client={client()}/>);

    expect(await screen.findByRole('heading', { name: 'Registered profiles' })).toBeInTheDocument();
    expect(screen.getByText('Default')).toBeInTheDocument();
    expect(screen.queryByText('Identity')).not.toBeInTheDocument();
    expect(screen.getAllByText('C:\\Users\\user\\.codex')).toHaveLength(2);

    await screen.findByRole('button', { name: /C:\/Orchid\/profile-two/ });
    await user.click(screen.getByRole('button', { name: /C:\/Orchid\/profile-two/ }));
    expect(await screen.findByText('Orchid-managed profile')).toBeInTheDocument();
  });

  it('adds profiles through the guided modal and nested discovery picker', async () => {
    const user = userEvent.setup();
    render(<NativeProfileSettings client={client()}/>);

    await user.click(screen.getByRole('button', { name: 'Add Codex profile' }));
    const dialog = screen.getByRole('dialog', { name: 'Add Codex profile' });
    const address = within(dialog).getByRole('textbox', { name: 'Codex home folder' });
    await user.click(within(dialog).getByRole('checkbox', { name: 'Create new Codex home folder' }));
    expect(address).toBeDisabled();
    await user.click(within(dialog).getByRole('checkbox', { name: 'Create new Codex home folder' }));
    await user.click(within(dialog).getByRole('button', { name: 'Discover existing home folders' }));

    const picker = await screen.findByRole('dialog', { name: 'Discovered Codex home folders' });
    await user.click(within(picker).getByRole('button', { name: 'Use this folder' }));
    expect(address).toHaveValue('C:/Users/user/.codex-review');
  });

  it('keeps login verification and browser login inside the login operation', async () => {
    const user = userEvent.setup();
    const refreshReadiness = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles }));
    const requestLogin = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles }));
    render(<NativeProfileSettings client={client({ refreshReadiness, requestLogin })}/>);

    await screen.findByRole('button', { name: /C:\/Orchid\/profile-two/ });
    await user.click(screen.getByRole('button', { name: /C:\/Orchid\/profile-two/ }));
    await user.click(screen.getByRole('button', { name: 'Verify login' }));
    const dialog = screen.getByRole('dialog', { name: 'Verify login' });
    await user.click(within(dialog).getByRole('button', { name: 'Verify login' }));
    await waitFor(() => expect(refreshReadiness).toHaveBeenCalledWith('p2'));
    await user.click(within(dialog).getByRole('button', { name: 'Request browser login' }));
    await waitFor(() => expect(requestLogin).toHaveBeenCalledWith('p2'));
  });
});
