import type { AgentIdentity } from '../application/agentSessions';
import {
  assignedIdentityFromLegacyAgentIdentity,
  legacyHarnessRoleLabel,
} from '../application/identities';
import { AgentIdentityBadge as IdentityBadge } from '../features/identities';
import type { CSSProperties } from 'react';
import './agentIdentityBadge.css';

export interface AgentIdentityBadgeProps {
  readonly identity: AgentIdentity;
  readonly compact?: boolean;
}

/** Compatibility wrapper for callers that still receive the legacy AgentIdentity DTO. */
export function AgentIdentityBadge({ identity, compact = false }: AgentIdentityBadgeProps) {
  const assignedIdentity = assignedIdentityFromLegacyAgentIdentity(identity);
  const role = legacyHarnessRoleLabel(identity.harnessRole);
  // Legacy selectors and metadata stay at this boundary until wire consumers use identities.
  return (
    <span
      className={`agent-identity-badge agent-identity-marker is-${assignedIdentity.shape}${
        compact ? ' is-compact' : ''
      }`}
      data-harness-role={identity.harnessRole}
      data-visual-identity-token={identity.visualIdentityToken}
      style={
        {
          '--agent-identity-accent': assignedIdentity.color,
        } as CSSProperties
      }
    >
      <IdentityBadge identity={assignedIdentity} secondaryLabel={role} compact={compact} />
    </span>
  );
}
