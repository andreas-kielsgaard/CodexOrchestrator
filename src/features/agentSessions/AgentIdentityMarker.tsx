import type { AgentIdentity } from '../../application/agentSessions';

export interface AgentIdentityMarkerProps {
  readonly identity: AgentIdentity;
}

/** Presentation-only marker for a session-owned identity. */
export function AgentIdentityMarker({ identity }: AgentIdentityMarkerProps) {
  return (
    <span
      className="agent-identity-marker"
      data-harness-role={identity.harnessRole}
      data-visual-identity-token={identity.visualIdentityToken}
      aria-hidden="true"
    >
      {identity.name.trim().charAt(0).toLocaleUpperCase()}
    </span>
  );
}
