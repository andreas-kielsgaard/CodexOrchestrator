import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NativeProfileSettings } from './NativeProfileSettings';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';

const readiness = { authentication: 'unknown' as const, sandboxInitialization: 'unknown' as const, workspaceWriteCanary: 'not_run' as const, mcpReporting: 'not_assessed' as const, attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } };
const profiles: readonly NativeProfile[] = [
  { id: 'p1', homePath: 'C:/one', ownership: 'registered_existing', lifecycle: 'active', selected: true, readiness },
  { id: 'p2', homePath: 'C:/two', ownership: 'application_dedicated', lifecycle: 'active', selected: false, readiness },
];

function client(overrides: Partial<NativeProfileClient> = {}): NativeProfileClient {
  const query = async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles });
  return { load: query, registerExisting: query, createDedicated: query, select: query, requestLogin: query, refreshReadiness: query, initializeSandbox: query, runCanary: query, probeMcp: query, ...overrides };
}

describe('NativeProfileSettings', () => {
  it('targets the clicked non-selected card and keeps setup actions per-card', async () => {
    const user = userEvent.setup();
    const select = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles: profiles.map((profile) => ({ ...profile, selected: profile.id === 'p2' })) }));
    const refreshReadiness = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles }));
    render(<NativeProfileSettings client={client({ select, refreshReadiness })} />);
    await screen.findByText('C:/two');
    const availableSelect = screen.getAllByRole('button', { name: 'Select' }).find((button) => !button.hasAttribute('disabled'));
    expect(availableSelect).toBeDefined();
    await user.click(availableSelect!);
    await waitFor(() => expect(select).toHaveBeenCalledWith('p2'));
    await user.click(screen.getAllByRole('button', { name: 'Refresh login status' })[1]);
    await waitFor(() => expect(refreshReadiness).toHaveBeenCalledWith('p2'));
  });
});
