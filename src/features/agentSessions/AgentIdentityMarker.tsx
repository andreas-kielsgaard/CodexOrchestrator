import type { AgentIdentity } from '../../application/agentSessions';
import { assignedIdentityFromLegacyAgentIdentity } from '../../application/identities';
import { AgentIdentityMarker as IdentityMarker } from '../identities';
import type { CSSProperties } from 'react';
import './agentIdentityMarker.css';

export interface AgentIdentityMarkerProps {
  readonly identity: AgentIdentity;
}

/** Compatibility wrapper for callers that still receive the legacy AgentIdentity DTO. */
export function AgentIdentityMarker({ identity }: AgentIdentityMarkerProps) {
  const assignedIdentity = assignedIdentityFromLegacyAgentIdentity(identity);
  return (
    <span
      className={`agent-identity-marker is-${assignedIdentity.shape}`}
      data-harness-role={identity.harnessRole}
      data-visual-identity-token={identity.visualIdentityToken}
      style={
        {
          '--agent-identity-accent': assignedIdentity.color,
        } as CSSProperties
      }
      aria-hidden="true"
    >
      <IdentityMarker identity={assignedIdentity} />
    </span>
  );
}
