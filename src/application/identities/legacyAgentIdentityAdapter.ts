import type { AgentIdentity } from '../agentSessions';
import type { AssignedAgentIdentity } from './contracts';

const LEGACY_DEFAULT_IDENTITY_COLOR = '#e8ece8';

/** Temporary boundary for presentation consumers that still receive the legacy Session DTO. */
export function assignedIdentityFromLegacyAgentIdentity(
  identity: AgentIdentity,
): AssignedAgentIdentity {
  return {
    originIdentityId: null,
    displayName: identity.name,
    color: identity.visualIdentityAccent ?? LEGACY_DEFAULT_IDENTITY_COLOR,
    shape: identity.visualIdentityShape ?? 'circle',
  };
}

export function legacyHarnessRoleLabel(harnessRole: string): string {
  return harnessRole
    .split('_')
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`)
    .join(' ');
}
