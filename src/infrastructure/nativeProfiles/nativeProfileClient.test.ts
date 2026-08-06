import { describe, expect, it } from 'vitest';
import { decodeNativeProfileQuery, createNativeProfileClient } from './nativeProfileClient';

const profile = {
  id: 'p1', homePath: 'C:/codex', ownership: 'registered_existing', lifecycle: 'active', selected: true,
  readiness: { authentication: 'unknown', sandboxInitialization: 'unknown', workspaceWriteCanary: 'not_run', mcpReporting: 'not_assessed', attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } },
};
const query = () => ({ contract: 'native-codex-profile-query/v1', profiles: [profile] });

describe('native profile client', () => {
  it('rejects unknown fields and authority-bearing enum values', () => {
    expect(() => decodeNativeProfileQuery({ ...query(), extra: true })).toThrow(/unknown field/);
    expect(() => decodeNativeProfileQuery({ ...query(), profiles: [{ ...profile, ownership: 'admin' }] })).toThrow(/ownership/);
  });
  it('serializes actions and reloads durable state after each action', async () => {
    const calls: string[] = [];
    const invoke = async <T>(command: string) => { calls.push(command); return (command === 'load_native_profile_query' ? query() : profile) as T; };
    const client = createNativeProfileClient(invoke);
    await Promise.all([client.select('p1'), client.refreshReadiness('p1')]);
    expect(calls.filter((call) => call === 'load_native_profile_query')).toHaveLength(2);
    expect(calls.indexOf('select_native_profile')).toBeLessThan(calls.indexOf('refresh_native_profile_readiness'));
  });
});
