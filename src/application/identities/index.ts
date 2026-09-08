export type {
  AssignedAgentIdentity,
  IdentityDefinition,
  IdentityId,
  IdentityShape,
} from './contracts';
export {
  assignedIdentityFromLegacyAgentIdentity,
  legacyHarnessRoleLabel,
} from './legacyAgentIdentityAdapter';
export type {
  CreateIdentityInput,
  DeleteIdentityInput,
  IdentityCatalogEntry,
  IdentityManagementClient,
  UpdateIdentityInput,
} from './management';
export { identityDefinitionFromCatalog } from './management';
