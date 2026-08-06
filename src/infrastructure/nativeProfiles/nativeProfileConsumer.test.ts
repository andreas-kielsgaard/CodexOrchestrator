import { describe, expect, it } from 'vitest';
import { createNativeProfileApplicationConsumer, resolveNativeProfileApplicationConsumer } from './nativeProfileConsumer';
import type { NativeProfileQuery } from './nativeProfileClient';

const query: NativeProfileQuery = {
  contract: 'native-codex-profile-query/v1',
  profiles: [{
    id: 'p1', homePath: 'C:/codex', ownership: 'registered_existing', lifecycle: 'active', selected: true,
    readiness: { authentication: 'unknown', sandboxInitialization: 'unknown', workspaceWriteCanary: 'not_run', mcpReporting: 'not_assessed', attentions: { authentication: null, sandbox: null, canary: null, mcpReporting: null, continuity: null, cli: null } },
  }],
};

describe('native profile application consumer', () => {
  it('projects only the validated home and separate readiness facts', () => {
    expect(resolveNativeProfileApplicationConsumer(query, 'p1')).toEqual({
      profileId: 'p1', codexHome: 'C:/codex', readiness: query.profiles[0].readiness,
    });
  });

  it('rejects an id that is not the current selected profile', () => {
    expect(() => resolveNativeProfileApplicationConsumer(query, 'other')).toThrow(/not present/);
    expect(() => resolveNativeProfileApplicationConsumer({ ...query, profiles: [{ ...query.profiles[0], selected: false }] }, 'p1')).toThrow(/currently validated/);
  });

  it('loads current durable state before resolving the application boundary', async () => {
    const load = async () => query;
    await expect(createNativeProfileApplicationConsumer({ load }).resolve('p1')).resolves.toMatchObject({ codexHome: 'C:/codex' });
  });
});
