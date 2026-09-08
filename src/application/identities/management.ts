import type { IdentityDefinition, IdentityId, IdentityShape } from './contracts';

export interface IdentityCatalogEntry extends IdentityDefinition {
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface CreateIdentityInput {
  readonly displayName: string;
  readonly color: string;
  readonly shape: IdentityShape;
}

export interface UpdateIdentityInput extends CreateIdentityInput {
  readonly identityId: IdentityId;
}

export interface DeleteIdentityInput {
  readonly identityId: IdentityId;
}

export interface IdentityManagementClient {
  list(): Promise<readonly IdentityCatalogEntry[]>;
  create(input: CreateIdentityInput): Promise<IdentityCatalogEntry>;
  update(input: UpdateIdentityInput): Promise<IdentityCatalogEntry>;
  delete(input: DeleteIdentityInput): Promise<void>;
}

/** Removes catalog persistence metadata before an identity is consumed by product behavior. */
export function identityDefinitionFromCatalog(entry: IdentityCatalogEntry): IdentityDefinition {
  return {
    id: entry.id,
    displayName: entry.displayName,
    color: entry.color,
    shape: entry.shape,
  };
}
