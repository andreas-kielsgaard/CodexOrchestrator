import { render, screen, waitFor, within } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NativeProfileSettings } from './NativeProfileSettings';
import type { NativeProfile, NativeProfileClient } from '../../infrastructure/nativeProfiles/nativeProfileClient';

const readiness = { authentication: 'unknown' as const, sandboxInitialization: 'unknown' as const, workspaceWriteCanary: 'not_run' as const, dangerFullAccessCanary: 'not_run' as const, mcpReporting: 'not_assessed' as const, attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } };
const execution = { selectedMode: 'workspace_write' as const, dangerFullAccessAuthorized: false };
const loginAttempt = { disposition: 'not_requested' as const, browserHandoff: 'unobserved' as const, requestedAt: null, launchAcceptedAt: null, settledAt: null };
const setupAttempt = { phase: 'not_requested' as const, disposition: 'not_requested' as const, executable: null, version: null, workspaceSandboxSupported: null, correlationId: null, requestedAt: null, launchAcceptedAt: null, deadlineAt: null, settledAt: null, terminalClassification: 'not_observed' as const, terminalExitCode: null };
const profiles: readonly NativeProfile[] = [
  { id: 'p1', homePath: 'C:/one', ownership: 'registered_existing', lifecycle: 'active', selected: true, execution, loginAttempt, setupAttempt, readiness },
  { id: 'p2', homePath: 'C:/two', ownership: 'application_dedicated', lifecycle: 'active', selected: false, execution, loginAttempt, setupAttempt, readiness },
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

  it('shows durable login disposition without treating a request return as browser or authentication proof', async () => {
    const user = userEvent.setup();
    const requestedProfiles = profiles.map((profile) => profile.id === 'p1' ? { ...profile, loginAttempt: { disposition: 'pending' as const, browserHandoff: 'unobserved' as const, requestedAt: '2026-08-07T12:00:00Z', launchAcceptedAt: '2026-08-07T12:00:01Z', settledAt: null } } : profile);
    const requestLogin = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles: requestedProfiles }));
    render(<NativeProfileSettings client={client({ requestLogin })} />);
    const card = await screen.findByRole('article', { name: 'Codex home p1' });
    await user.click(within(card).getByRole('button', { name: 'Request browser login' }));
    await waitFor(() => expect(requestLogin).toHaveBeenCalledWith('p1'));
    expect(within(card).getByText('Login process launch was accepted; browser handoff is unobserved.')).toBeInTheDocument();
    expect(within(card).getByText('Process activity, browser handoff, and authentication are separate facts.')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Browser login request returned; review the durable login-attempt state.');
  });

  it('shows durable setup evidence without treating a request return as launch, UAC, or readiness proof', async () => {
    const user = userEvent.setup();
    const requestedProfiles = profiles.map((profile) => profile.id === 'p1' ? { ...profile, setupAttempt: { phase: 'sandbox_initialization' as const, disposition: 'terminal_failed' as const, executable: 'C:/application-owned/codex.exe', version: 'codex-cli test', workspaceSandboxSupported: true, correlationId: 'native-setup-correlation', requestedAt: '2026-08-07T12:00:00Z', launchAcceptedAt: '2026-08-07T12:00:01Z', deadlineAt: '2026-08-07T12:02:00Z', settledAt: '2026-08-07T12:00:02Z', terminalClassification: 'exit_code' as const, terminalExitCode: 7 } } : profile);
    const initializeSandbox = vi.fn(async () => ({ contract: 'native-codex-profile-query/v1' as const, profiles: requestedProfiles }));
    render(<NativeProfileSettings client={client({ initializeSandbox })} />);
    const card = await screen.findByRole('article', { name: 'Codex home p1' });
    await user.click(within(card).getByRole('button', { name: 'Request sandbox initialization' }));
    await waitFor(() => expect(initializeSandbox).toHaveBeenCalledWith('p1'));
    expect(within(card).getByText('Native setup process ended unsuccessfully.')).toBeInTheDocument();
    expect(within(card).getByText('Terminal classification')).toBeInTheDocument();
    expect(within(card).getByText('Request return, child launch, terminal outcome, UAC confirmation, sandbox initialization, and canary readiness are separate facts.')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Sandbox initialization request returned; review the durable setup-attempt facts.');
  });

  it('contains the reported short viewport in an internal scroll region with narrow wrapping', async () => {
    render(<NativeProfileSettings client={client()} />);
    const settings = await screen.findByRole('main', { name: 'Technical Codex settings' });
    expect(settings).toHaveClass('native-profile-settings');
    expect(settings).toHaveAttribute('tabindex', '0');
    const styles = readFileSync(resolve('src/features/nativeProfiles/nativeProfileSettings.css'), 'utf8');
    const shell = readFileSync(resolve('src/styles.css'), 'utf8');
    expect(shell).toMatch(/\.primary-app-shell\s*\{[\s\S]*height: 100vh;[\s\S]*padding-top: 48px;[\s\S]*overflow: hidden;/);
    expect(styles).toMatch(/\.native-profile-settings\s*\{[\s\S]*height: 100%;[\s\S]*min-height: 0;[\s\S]*overflow-x: hidden;[\s\S]*overflow-y: auto;[\s\S]*overscroll-behavior: contain;/);
    expect(styles).toMatch(/@media \(max-width: 520px\)\s*\{[\s\S]*\.native-profile-facts, \.native-profile-login-attempt dl\s*\{[\s\S]*grid-template-columns: minmax\(0, 1fr\);/);
  });
});
