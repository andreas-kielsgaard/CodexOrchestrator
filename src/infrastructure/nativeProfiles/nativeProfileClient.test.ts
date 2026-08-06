import { describe, expect, it } from 'vitest';
import { decodeNativeProfileQuery, createNativeProfileClient } from './nativeProfileClient';

const profile = {
  id: 'p1', homePath: 'C:/codex', ownership: 'registered_existing', lifecycle: 'active', selected: true,
  execution: { selectedMode: 'workspace_write', dangerFullAccessAuthorized: false },
  loginAttempt: { disposition: 'not_requested', browserHandoff: 'unobserved', requestedAt: null, launchAcceptedAt: null, settledAt: null },
  setupAttempt: { phase: 'not_requested', disposition: 'not_requested', executable: null, version: null, workspaceSandboxSupported: null, correlationId: null, requestedAt: null, launchAcceptedAt: null, deadlineAt: null, settledAt: null, terminalClassification: 'not_observed', terminalExitCode: null },
  readiness: { authentication: 'unknown', sandboxInitialization: 'unknown', workspaceWriteCanary: 'not_run', dangerFullAccessCanary: 'not_run', mcpReporting: 'not_assessed', attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } },
};
const query = () => ({ contract: 'native-codex-profile-query/v1', profiles: [profile] });

describe('native profile client', () => {
  it('rejects unknown fields and authority-bearing enum values', () => {
    expect(() => decodeNativeProfileQuery({ ...query(), extra: true })).toThrow(/unknown field/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, ownership: 'admin' }] })).toThrow(/ownership/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [profile, { ...profile, id: 'p2' }] })).toThrow(/multiple selected/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, id: 'p1' }, { ...profile, selected: false }] })).toThrow(/duplicate profile ids/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, lifecycle: 'missing_or_moved' }] })).toThrow(/stale/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, homePath: 'relative/home' }] })).toThrow(/absolute/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, readiness: { ...profile.readiness, attentions: { ...profile.readiness.attentions, cli: ' ' } } }] })).toThrow(/attention/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, loginAttempt: { ...profile.loginAttempt, browserHandoff: 'observed' } }] })).toThrow(/browser handoff/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, loginAttempt: { ...profile.loginAttempt, extra: true } }] })).toThrow(/login attempt.*unknown field/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, terminalExitCode: 1.5 } }] })).toThrow(/terminal exit code/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, workspaceSandboxSupported: 'yes' } }] })).toThrow(/workspace sandbox capability/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, opaqueSandboxState: 'private' } }] })).toThrow(/setup attempt.*unknown field/);
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, disposition: 'legacy_unclassified_failed', terminalClassification: 'legacy_unclassified_failed' } }] }).profiles[0].setupAttempt).toMatchObject({ disposition: 'legacy_unclassified_failed', terminalClassification: 'legacy_unclassified_failed' });
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, disposition: 'policy_unsupported', terminalClassification: 'policy_unsupported', workspaceSandboxSupported: false } }] }).profiles[0].setupAttempt).toMatchObject({ disposition: 'policy_unsupported', terminalClassification: 'policy_unsupported', workspaceSandboxSupported: false });
  });
  it('serializes actions and reloads durable state after each action', async () => {
    const calls: string[] = [];
    const invoke = async <T>(command: string) => { calls.push(command); return (command === 'load_native_profile_query' ? query() : profile) as T; };
    const client = createNativeProfileClient(invoke);
    await Promise.all([client.select('p1'), client.refreshReadiness('p1')]);
    expect(calls.filter((call) => call === 'load_native_profile_query')).toHaveLength(2);
    expect(calls.indexOf('select_native_profile')).toBeLessThan(calls.indexOf('refresh_native_profile_readiness'));
  });

  it('rejects malformed action DTOs before reloading durable state', async () => {
    const invoke = async <T>(command: string) => (command === 'load_native_profile_query' ? query() : { ...profile, extra: true }) as T;
    await expect(createNativeProfileClient(invoke).select('p1')).rejects.toThrow(/unknown field/);
  });
  it('orders a public load behind an in-flight action', async () => {
    const calls: string[] = [];
    let releaseAction!: () => void;
    const actionGate = new Promise<void>((resolve) => { releaseAction = resolve; });
    const invoke = async <T>(command: string) => {
      calls.push(command);
      if (command === 'select_native_profile') await actionGate;
      return (command === 'load_native_profile_query' ? query() : profile) as T;
    };
    const client = createNativeProfileClient(invoke);
    const action = client.select('p1');
    const load = client.load();
    await Promise.resolve();
    expect(calls).toEqual(['select_native_profile']);
    releaseAction();
    await Promise.all([action, load]);
    expect(calls).toEqual(['select_native_profile', 'load_native_profile_query', 'load_native_profile_query']);
  });
});
