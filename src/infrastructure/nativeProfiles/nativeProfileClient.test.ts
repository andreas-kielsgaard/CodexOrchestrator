import { describe, expect, it } from 'vitest';
import { decodeNativeProfileQuery, createNativeProfileClient } from './nativeProfileClient';

const profile = {
  id: 'p1', homePath: 'C:/codex', ownership: 'registered_existing', lifecycle: 'active', selected: true,
  execution: { selectedMode: 'workspace_write', dangerFullAccessAuthorized: false, dangerAuthorization: { disposition: 'not_authorized', authorityScope: null, authorityVersion: null, correlationId: null, authorizedAt: null, revokedAt: null } },
  loginAttempt: { disposition: 'not_requested', browserHandoff: 'unobserved', requestedAt: null, launchAcceptedAt: null, settledAt: null },
  setupAttempt: { phase: 'not_requested', disposition: 'not_requested', executable: null, version: null, workspaceSandboxSupported: null, correlationId: null, requestedAt: null, launchAcceptedAt: null, deadlineAt: null, settledAt: null, terminalClassification: 'not_observed', terminalExitCode: null },
  sandboxAdoption: { disposition: 'not_verified', executable: null, version: null, workspaceSandboxSupported: null, windowsSandboxSetupSupported: null, correlationId: null, observedAt: null, elevatedModeObserved: null },
  sandboxAdoptionConfirmation: { disposition: 'not_confirmed', correlationId: null, confirmedAt: null },
  fullAccessCanaryAttempt: { disposition: 'not_requested', authorizationVersion: null, authorizationCorrelationId: null, correlationId: null, requestedAt: null, launchAcceptedAt: null, deadlineAt: null, settledAt: null, processActivity: 'unobserved', providerActivity: 'unobserved', terminalClassification: 'not_observed', terminalExitCode: null, receiptObserved: false, cleanupDisposition: 'not_observed' },
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
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoption: { ...profile.sandboxAdoption, opaqueSandboxState: 'private' } }] })).toThrow(/sandbox adoption.*unknown field/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoptionConfirmation: { ...profile.sandboxAdoptionConfirmation, opaqueState: 'private' } }] })).toThrow(/confirmation.*unknown field/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, execution: { ...profile.execution, dangerAuthorization: { ...profile.execution.dangerAuthorization, authorityScope: 'full_machine_filesystem_and_unrestricted_network', authorityVersion: 'danger-full-access/unrestricted-network/v1', correlationId: 'fresh', authorizedAt: '2026-08-07T12:00:00Z' } } }] })).toThrow(/danger authorization/);
    const currentDangerAuthorization = { disposition: 'authorized', authorityScope: 'full_machine_filesystem_and_unrestricted_network', authorityVersion: 'danger-full-access/unrestricted-network/v1', correlationId: 'native-danger-authorization', authorizedAt: '2026-08-07T12:00:00Z', revokedAt: null };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, execution: { selectedMode: 'danger_full_access', dangerFullAccessAuthorized: true, dangerAuthorization: currentDangerAuthorization } }] }).profiles[0].execution.dangerAuthorization).toMatchObject({ disposition: 'authorized' });
    const passedFullAccessCanary = { disposition: 'passed', authorizationVersion: 'danger-full-access/unrestricted-network/v1', authorizationCorrelationId: 'native-danger-authorization', correlationId: 'native-full-access-canary', requestedAt: '2026-08-07T12:00:00Z', launchAcceptedAt: '2026-08-07T12:00:01Z', deadlineAt: '2026-08-07T12:02:00Z', settledAt: '2026-08-07T12:00:02Z', processActivity: 'terminal_observed', providerActivity: 'unobserved', terminalClassification: 'exit_code', terminalExitCode: 0, receiptObserved: true, cleanupDisposition: 'removed' };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, fullAccessCanaryAttempt: passedFullAccessCanary }] }).profiles[0].fullAccessCanaryAttempt).toMatchObject({ disposition: 'passed', cleanupDisposition: 'removed' });
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, fullAccessCanaryAttempt: { ...passedFullAccessCanary, cleanupDisposition: 'failed' } }] })).toThrow(/passed full-access canary/);
    for (const contradiction of [
      { launchAcceptedAt: '2026-08-07T11:59:59Z' },
      { deadlineAt: null },
      { settledAt: '2026-08-07T12:00:00Z' },
      { terminalClassification: 'receipt_missing' },
      { terminalExitCode: null },
      { terminalClassification: 'not_observed' },
      { disposition: 'terminal_failed', receiptObserved: true, terminalClassification: 'receipt_missing' },
      { disposition: 'cancelled', processActivity: 'terminal_observed', terminalClassification: 'cancelled', terminalExitCode: null, receiptObserved: false, cleanupDisposition: 'removed' },
      { disposition: 'cleanup_failed', terminalClassification: 'cleanup_failed', cleanupDisposition: 'failed', receiptObserved: false },
    ]) expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, fullAccessCanaryAttempt: { ...passedFullAccessCanary, ...contradiction } }] })).toThrow(/full-access canary/);
    const legacyFullAccessCanary = { disposition: 'legacy_unverified', authorizationVersion: null, authorizationCorrelationId: null, correlationId: null, requestedAt: '2026-08-07T12:00:00Z', launchAcceptedAt: null, deadlineAt: null, settledAt: '2026-08-07T12:00:02Z', processActivity: 'unobserved', providerActivity: 'unobserved', terminalClassification: 'legacy_unverified', terminalExitCode: null, receiptObserved: false, cleanupDisposition: 'not_observed' };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, fullAccessCanaryAttempt: legacyFullAccessCanary }] }).profiles[0].fullAccessCanaryAttempt).toMatchObject({ disposition: 'legacy_unverified' });
    for (const contradiction of [{ deadlineAt: '2026-08-07T12:02:00Z' }, { correlationId: 'invented' }, { processActivity: 'launch_accepted' }, { terminalExitCode: 1 }, { receiptObserved: true }])
      expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, fullAccessCanaryAttempt: { ...legacyFullAccessCanary, ...contradiction } }] })).toThrow(/legacy full-access canary/);
    const verifiedAdoption = { disposition: 'verified', executable: 'C:/application-owned/codex.exe', version: 'codex-cli test', workspaceSandboxSupported: true, windowsSandboxSetupSupported: true, correlationId: 'native-adoption-correlation', observedAt: '2026-08-07T12:00:00Z', elevatedModeObserved: true };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoption: verifiedAdoption }] }).profiles[0].sandboxAdoption).toMatchObject({ disposition: 'verified', elevatedModeObserved: true });
    for (const contradiction of [
      { workspaceSandboxSupported: false },
      { windowsSandboxSetupSupported: false },
      { elevatedModeObserved: false },
      { observedAt: 'not-a-timestamp' },
      { executable: null },
    ]) expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoption: { ...verifiedAdoption, ...contradiction } }] })).toThrow(/sandbox adoption/);
    const confirmedAdoption = { disposition: 'confirmed', correlationId: 'native-adoption-confirmation', confirmedAt: '2026-08-07T12:01:00Z' };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoptionConfirmation: confirmedAdoption }] }).profiles[0].sandboxAdoptionConfirmation).toMatchObject({ disposition: 'confirmed' });
    for (const contradiction of [{ correlationId: null }, { confirmedAt: null }, { confirmedAt: 'not-a-timestamp' }])
      expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, sandboxAdoptionConfirmation: { ...confirmedAdoption, ...contradiction } }] })).toThrow(/confirmation/);
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...profile.setupAttempt, disposition: 'legacy_unclassified_failed', terminalClassification: 'legacy_unclassified_failed' } }] }).profiles[0].setupAttempt).toMatchObject({ disposition: 'legacy_unclassified_failed', terminalClassification: 'legacy_unclassified_failed' });
    const receiptMissing = { ...profile.setupAttempt, phase: 'workspace_write_canary', disposition: 'terminal_failed', executable: 'C:/application-owned/codex.exe', version: 'codex-cli test', workspaceSandboxSupported: true, correlationId: 'native-canary-correlation', requestedAt: '2026-08-07T12:00:00Z', launchAcceptedAt: '2026-08-07T12:00:01Z', deadlineAt: '2026-08-07T12:02:00Z', settledAt: '2026-08-07T12:00:02Z', terminalClassification: 'receipt_missing', terminalExitCode: 1 };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: receiptMissing }] }).profiles[0].setupAttempt).toMatchObject({ disposition: 'terminal_failed', terminalClassification: 'receipt_missing', terminalExitCode: 1 });
    for (const contradiction of [{ phase: 'sandbox_initialization' }, { disposition: 'terminal_succeeded' }, { workspaceSandboxSupported: false }, { launchAcceptedAt: null }, { settledAt: null }, { executable: null }, { correlationId: null }])
      expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...receiptMissing, ...contradiction } }] })).toThrow(/receipt_missing/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...receiptMissing, terminalClassification: 'private_failure' } }] })).toThrow(/setup terminal classification/);
    const policyUnsupported = { ...profile.setupAttempt, phase: 'sandbox_initialization', disposition: 'policy_unsupported', executable: 'C:/application-owned/codex.exe', version: 'codex-cli test', workspaceSandboxSupported: false, correlationId: 'native-setup-correlation', requestedAt: '2026-08-07T12:00:00Z', deadlineAt: '2026-08-07T12:02:00Z', settledAt: '2026-08-07T12:00:00Z', terminalClassification: 'policy_unsupported', terminalExitCode: null };
    expect(decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: policyUnsupported }] }).profiles[0].setupAttempt).toMatchObject({ disposition: 'policy_unsupported', terminalClassification: 'policy_unsupported', workspaceSandboxSupported: false });
    for (const contradiction of [
      { launchAcceptedAt: '2026-08-07T12:00:01Z' },
      { workspaceSandboxSupported: true },
      { terminalExitCode: 1 },
      { terminalClassification: 'exit_code' },
      { phase: 'not_requested' },
      { executable: null },
      { version: null },
      { correlationId: null },
      { requestedAt: null },
      { deadlineAt: null },
      { settledAt: null },
      { settledAt: 'not-a-timestamp' },
    ]) expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, setupAttempt: { ...policyUnsupported, ...contradiction } }] })).toThrow(/policy_unsupported/);
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
