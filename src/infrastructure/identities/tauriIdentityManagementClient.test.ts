import {
  createTauriIdentityManagementClient,
  decodeIdentityCatalog,
  decodeIdentityCatalogEntry,
} from './tauriIdentityManagementClient';

const avery = {
  id: 'identity-avery',
  displayName: 'Avery',
  color: '#39745a',
  shape: 'circle',
  createdAt: '2026-08-27T09:00:00Z',
  updatedAt: '2026-08-27T09:00:00Z',
} as const;

describe('Tauri Identity management client', () => {
  it('uses the canonical CRUD commands and explicit input objects', async () => {
    const calls: { command: string; args?: Record<string, unknown> }[] = [];
    const invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      const result: Record<string, unknown> = {
        list_identities: [avery],
        create_identity: avery,
        update_identity: {
          ...avery,
          displayName: 'Avery Stone',
          color: '#112233',
          shape: 'hexagon',
          updatedAt: '2026-08-27T10:00:00Z',
        },
        delete_identity: null,
      };
      return result[command] as T;
    };
    const client = createTauriIdentityManagementClient(invoke);

    await client.list();
    await client.create({ displayName: 'Avery', color: '#39745A', shape: 'circle' });
    await client.update({
      identityId: 'identity-avery',
      displayName: 'Avery Stone',
      color: '#112233',
      shape: 'hexagon',
    });
    await client.delete({ identityId: 'identity-avery' });

    expect(calls).toEqual([
      { command: 'list_identities', args: undefined },
      {
        command: 'create_identity',
        args: {
          input: { displayName: 'Avery', color: '#39745a', shape: 'circle' },
        },
      },
      {
        command: 'update_identity',
        args: {
          input: {
            identityId: 'identity-avery',
            displayName: 'Avery Stone',
            color: '#112233',
            shape: 'hexagon',
          },
        },
      },
      {
        command: 'delete_identity',
        args: { input: { identityId: 'identity-avery' } },
      },
    ]);
  });

  it('strictly decodes definitions, timestamps, colors, and unique IDs', () => {
    expect(decodeIdentityCatalogEntry(avery)).toEqual(avery);
    expect(() => decodeIdentityCatalog([{ ...avery }, { ...avery }])).toThrow(/unique/);
    expect(() => decodeIdentityCatalogEntry({ ...avery, providerId: 'global-provider' })).toThrow(
      /unknown field/,
    );
    expect(() => decodeIdentityCatalogEntry({ ...avery, color: 'green' })).toThrow(/hexadecimal/);
    expect(() =>
      decodeIdentityCatalogEntry({
        ...avery,
        updatedAt: '2026-08-27T08:00:00Z',
      }),
    ).toThrow(/precedes/);
  });

  it('rejects request-response mismatches and non-null delete responses', async () => {
    const mismatched = createTauriIdentityManagementClient(async <T>(command: string) => {
      if (command === 'delete_identity') return { deleted: true } as T;
      return { ...avery, displayName: 'Different' } as T;
    });

    await expect(
      mismatched.create({ displayName: 'Avery', color: '#39745a', shape: 'circle' }),
    ).rejects.toThrow(/do not match/);
    await expect(mismatched.delete({ identityId: 'identity-avery' })).rejects.toThrow(/null/);
  });
});
