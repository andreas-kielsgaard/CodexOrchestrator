import {
  createHarnessId,
  createHarnessVersionNumber,
  createHarnessVersionRef,
} from '../../application/harnesses';
import { exampleHarnessConfiguration } from '../../application/harnesses/testFixtures';
import {
  createTauriHarnessManagementClient,
  decodeHarnessConfiguration,
  decodeHarnessDetails,
  decodeResolvedHarnessVersion,
} from './tauriHarnessManagementClient';

const configuration = exampleHarnessConfiguration();
const record = {
  id: 'harness-epic-plan-builder',
  metadata: { name: 'Epic Plan Builder' },
  createdAt: '2026-08-27T08:00:00Z',
  updatedAt: '2026-08-27T08:05:00Z',
};
const reference = (version: number) => ({
  harnessId: 'harness-epic-plan-builder',
  version,
});
const version = (number: number, scope: unknown = { kind: 'reusable' }) => ({
  reference: reference(number),
  scope,
  configuration,
  configurationDigest: 'a'.repeat(64),
  createdAt: '2026-08-27T08:10:00Z',
});
const draft = {
  harnessId: 'harness-epic-plan-builder',
  basedOnVersion: 1,
  configuration,
  draftRevision: 7,
  savedAt: '2026-08-27T08:08:00Z',
};

describe('Tauri Harness management client', () => {
  it('uses the canonical commands and explicit command DTOs', async () => {
    const calls: { command: string; args?: Record<string, unknown> }[] = [];
    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      const result: Record<string, unknown> = {
        list_harnesses: [record],
        load_harness: { harness: record, draft, versions: [version(1)], replacements: [] },
        create_harness: record,
        rename_harness: { ...record, metadata: { name: 'Plan Builder' } },
        save_harness_draft: draft,
        publish_harness_draft: version(2),
        publish_session_harness_override: version(3, {
          kind: 'session_specific',
          sessionId: 'session-1',
        }),
        order_harness_version_replacement: { source: reference(1), target: reference(2) },
        resolve_harness_version: {
          requested: reference(1),
          version: version(2),
          replacementPath: [reference(1), reference(2)],
        },
      };
      return result[command] as T;
    };
    const client = createTauriHarnessManagementClient(invoke);
    const harnessId = createHarnessId(record.id);
    const first = createHarnessVersionRef(harnessId, createHarnessVersionNumber(1));
    const second = createHarnessVersionRef(harnessId, createHarnessVersionNumber(2));

    await client.list();
    await client.load({ harnessId });
    await client.create({ name: 'Epic Plan Builder', initialConfiguration: configuration });
    await client.rename({ harnessId, name: 'Plan Builder' });
    await client.saveDraft({ harnessId, basedOn: first, configuration });
    await client.publishDraft({ harnessId });
    await client.publishSessionOverride({ harnessId, sessionId: 'session-1', configuration });
    await client.orderReplacement({ source: first, target: second });
    await client.resolveVersion({ requested: first });

    expect(calls.map(({ command }) => command)).toEqual([
      'list_harnesses',
      'load_harness',
      'create_harness',
      'rename_harness',
      'save_harness_draft',
      'publish_harness_draft',
      'publish_session_harness_override',
      'order_harness_version_replacement',
      'resolve_harness_version',
    ]);
    expect(calls[1]?.args).toEqual({ input: { harnessId } });
    expect(calls[4]?.args).toEqual({
      input: { harnessId, basedOn: first, configuration },
    });
    expect(calls[6]?.args).toEqual({
      input: { harnessId, sessionId: 'session-1', configuration },
    });
    expect(calls[7]?.args).toEqual({ input: { source: first, target: second } });
  });

  it('projects transport-only draft revisions and digests out of product DTOs', () => {
    const details = decodeHarnessDetails({
      harness: record,
      draft,
      versions: [version(1)],
      replacements: [],
    });

    expect(details.draft).toMatchObject({
      harnessId: record.id,
      basedOn: reference(1),
      savedAt: draft.savedAt,
    });
    expect(details.draft).not.toHaveProperty('draftRevision');
    expect(details.versions[0]).not.toHaveProperty('configurationDigest');
    expect(details.versions[0]).not.toHaveProperty('revisionId');
  });

  it('rejects unknown, malformed, or internally contradictory transport values', () => {
    expect(() =>
      decodeHarnessDetails({
        harness: record,
        draft: null,
        versions: [],
        replacements: [],
        revisionId: 'private',
      }),
    ).toThrow(/unknown field/);
    expect(() =>
      decodeHarnessConfiguration({
        ...configuration,
        runtime: { ...configuration.runtime, providerId: 'provider-private' },
      }),
    ).toThrow(/unknown field/);
    expect(() =>
      decodeHarnessDetails({
        harness: record,
        draft: null,
        versions: [
          {
            ...version(1),
            reference: { harnessId: 'different-harness', version: 1 },
          },
        ],
        replacements: [],
      }),
    ).toThrow(/different Harness/);
    expect(() =>
      decodeResolvedHarnessVersion({
        requested: reference(1),
        version: version(2),
        replacementPath: [reference(2)],
      }),
    ).toThrow(/start with the requested/);
    expect(() =>
      decodeResolvedHarnessVersion({
        requested: reference(1),
        version: version(2),
        replacementPath: [reference(1), reference(1), reference(2)],
      }),
    ).toThrow(/must be unique/);
  });

  it('rejects request-response mismatches at the transport boundary', async () => {
    const harnessId = createHarnessId(record.id);
    const client = createTauriHarnessManagementClient(async <T>(command: string) => {
      if (command === 'publish_session_harness_override') return version(2) as T;
      return {
        requested: reference(2),
        version: version(2),
        replacementPath: [reference(2)],
      } as T;
    });

    await expect(
      client.publishSessionOverride({ harnessId, sessionId: 'session-1', configuration }),
    ).rejects.toThrow(/does not match its request/);
    await expect(
      client.resolveVersion({
        requested: createHarnessVersionRef(harnessId, createHarnessVersionNumber(1)),
      }),
    ).rejects.toThrow(/does not match its request/);
  });
});
