import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NativeProfileSettings } from './NativeProfileSettings';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';

const readiness = { authentication: 'unknown' as const, sandboxInitialization: 'unknown' as const, workspaceWriteCanary: 'not_run' as const, dangerFullAccessCanary: 'not_run' as const, mcpReporting: 'not_assessed' as const, attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } };
const execution = { selectedMode: 'workspace_write' as const, dangerFullAccessAuthorized: false };
const profiles: readonly NativeProfile[] = [
  { id: 'p1', homePath: 'C:/one', ownership: 'registered_existing', lifecycle: 'active', selected: true, execution, readiness },
  { id: 'p2', homePath: 'C:/two', ownership: 'application_dedicated', lifecycle: 'active', selected: false, execution, readiness },
];

function client(overrides: Partial<NativeProfileClient> = {}): NativeProfileClient {
  const query = async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles });
  return { load: query, registerExisting: query, createDedicated: query, select: query, selectExecutionMode: query, authorizeDangerFullAccess: query, revokeDangerFullAccess: query, requestLogin: query, refreshReadiness: query, initializeSandbox: query, confirmSandboxInitialization: query, runCanary: query, runDangerFullAccessCanary: query, probeMcp: query, ...overrides };
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

  it('keeps danger authorization separate from mode selection and supports revocation', async () => {
    const user = userEvent.setup();
    const selectExecutionMode = vi.fn(async (_id: string, mode: 'workspace_write' | 'danger_full_access') => ({ contract: 'native-codex-profile-query/v1' as const, profiles: profiles.map((profile) => ({ ...profile, execution: { ...profile.execution, selectedMode: mode } })) }));
    const authorizeDangerFullAccess = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles: profiles.map((profile) => ({ ...profile, execution: { ...profile.execution, selectedMode: 'danger_full_access' as const, dangerFullAccessAuthorized: true } })) }));
    const revokeDangerFullAccess = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles }));
    render(<NativeProfileSettings client={client({ selectExecutionMode, authorizeDangerFullAccess, revokeDangerFullAccess })} />);
    const card = await screen.findByRole('article', { name: 'Codex home p1' });
    expect(within(card).getByText(/does not authorize itself/)).toBeInTheDocument();
    const danger = within(card).getByRole('radio', { name: /Danger Full Access/ });
    await user.click(danger);
    await waitFor(() => expect(selectExecutionMode).toHaveBeenCalledWith('p1', 'danger_full_access'));
    expect(within(card).getByRole('button', { name: 'Authorize Danger Full Access for this profile' })).toBeEnabled();
    await user.click(within(card).getByRole('button', { name: 'Authorize Danger Full Access for this profile' }));
    await waitFor(() => expect(authorizeDangerFullAccess).toHaveBeenCalledWith('p1'));
    expect(within(card).getByText('Authorization:')).toBeInTheDocument();
    const refreshedCard = await screen.findByRole('article', { name: 'Codex home p1' });
    expect(within(refreshedCard).getByRole('button', { name: 'Revoke Danger Full Access authorization' })).toBeInTheDocument();
  });

  it('reports rejected authorization without claiming it succeeded', async () => {
    const user = userEvent.setup();
    const selectExecutionMode = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles: profiles.map((profile) => ({ ...profile, execution: { ...profile.execution, selectedMode: 'danger_full_access' as const } })) }));
    const authorizeDangerFullAccess = vi.fn(async () => { throw new Error('Danger authorization was rejected.'); });
    render(<NativeProfileSettings client={client({ selectExecutionMode, authorizeDangerFullAccess })} />);
    const card = await screen.findByRole('article', { name: 'Codex home p1' });
    await user.click(within(card).getByRole('radio', { name: /Danger Full Access/ }));
    await user.click(within(card).getByRole('button', { name: 'Authorize Danger Full Access for this profile' }));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Danger Full Access authorization failed: Danger authorization was rejected.'));
    expect(within(card).getByText('not authorized')).toBeInTheDocument();
  });
});
