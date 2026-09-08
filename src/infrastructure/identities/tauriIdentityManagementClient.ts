import { invoke } from '@tauri-apps/api/core';
import type {
  CreateIdentityInput,
  IdentityCatalogEntry,
  IdentityId,
  IdentityManagementClient,
  IdentityShape,
  UpdateIdentityInput,
} from '../../application/identities';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriIdentityManagementClient(
  invokeCommand: Invoke = invoke,
): IdentityManagementClient {
  return {
    async list() {
      return decodeIdentityCatalog(await invokeCommand<unknown>('list_identities'));
    },
    async create(input) {
      const values = identityValues(input);
      const created = decodeIdentityCatalogEntry(
        await invokeCommand<unknown>('create_identity', { input: values }),
      );
      requireMatchingValues(created, values, 'Created Identity');
      return created;
    },
    async update(input) {
      const identityId = identityIdValue(input.identityId, 'Identity ID');
      const values = identityValues(input);
      const updated = decodeIdentityCatalogEntry(
        await invokeCommand<unknown>('update_identity', {
          input: { identityId, ...values },
        }),
      );
      if (updated.id !== identityId) throw new Error('Updated Identity does not match its request');
      requireMatchingValues(updated, values, 'Updated Identity');
      return updated;
    },
    async delete(input) {
      const identityId = identityIdValue(input.identityId, 'Identity ID');
      const result = await invokeCommand<unknown>('delete_identity', {
        input: { identityId },
      });
      if (result !== null) throw new Error('Delete Identity response must be null');
    },
  };
}

export const tauriIdentityManagementClient = createTauriIdentityManagementClient();

export function decodeIdentityCatalog(value: unknown): readonly IdentityCatalogEntry[] {
  if (!Array.isArray(value)) throw new Error('Identity catalog must be an array');
  const entries = value.map((entry, index) =>
    decodeIdentityCatalogEntry(entry, `Identity catalog entry ${index}`),
  );
  const ids = entries.map(({ id }) => id);
  if (new Set(ids).size !== ids.length) throw new Error('Identity catalog IDs must be unique');
  return entries;
}

export function decodeIdentityCatalogEntry(
  value: unknown,
  label = 'Identity definition',
): IdentityCatalogEntry {
  const root = exactObject(
    value,
    ['id', 'displayName', 'color', 'shape', 'createdAt', 'updatedAt'],
    label,
  );
  const createdAt = timestamp(root.createdAt, `${label} created timestamp`);
  const updatedAt = timestamp(root.updatedAt, `${label} updated timestamp`);
  if (Date.parse(updatedAt) < Date.parse(createdAt))
    throw new Error(`${label} updated timestamp precedes its creation`);
  return {
    id: identityIdValue(root.id, `${label} ID`),
    displayName: trimmedString(root.displayName, `${label} display name`),
    color: colorValue(root.color, `${label} color`),
    shape: shapeValue(root.shape, `${label} shape`),
    createdAt,
    updatedAt,
  };
}

function identityValues(input: CreateIdentityInput | UpdateIdentityInput): CreateIdentityInput {
  return {
    displayName: trimmedString(input.displayName, 'Identity display name'),
    color: colorValue(input.color, 'Identity color'),
    shape: shapeValue(input.shape, 'Identity shape'),
  };
}

function requireMatchingValues(
  entry: IdentityCatalogEntry,
  expected: CreateIdentityInput,
  label: string,
) {
  if (
    entry.displayName !== expected.displayName ||
    entry.color !== expected.color ||
    entry.shape !== expected.shape
  )
    throw new Error(`${label} values do not match its request`);
}

function exactObject(
  value: unknown,
  allowedKeys: readonly string[],
  label: string,
): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error(`${label} must be an object`);
  const object = value as Record<string, unknown>;
  for (const key of Object.keys(object))
    if (!allowedKeys.includes(key)) throw new Error(`${label} contains unknown field: ${key}`);
  for (const key of allowedKeys)
    if (!(key in object)) throw new Error(`${label} is missing field: ${key}`);
  return object;
}

function identityIdValue(value: unknown, label: string): IdentityId {
  return trimmedString(value, label);
}

function trimmedString(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.length === 0 || value.trim() !== value)
    throw new Error(`${label} must be a non-empty trimmed string`);
  return value;
}

function colorValue(value: unknown, label: string): string {
  if (typeof value !== 'string' || !/^#[0-9a-f]{6}$/i.test(value))
    throw new Error(`${label} must be a six-digit hexadecimal color`);
  return value.toLowerCase();
}

function shapeValue(value: unknown, label: string): IdentityShape {
  if (value !== 'circle' && value !== 'square' && value !== 'hexagon')
    throw new Error(`${label} is invalid`);
  return value;
}

function timestamp(value: unknown, label: string): string {
  if (
    typeof value !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value) ||
    Number.isNaN(Date.parse(value))
  )
    throw new Error(`${label} must be an RFC3339 timestamp`);
  return value;
}
