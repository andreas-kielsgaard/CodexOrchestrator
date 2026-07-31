import type { AgentIdentity } from '../application/agentSessions';
import { AgentIdentityMarker } from '../features/agentSessions/AgentIdentityMarker';
import './agentIdentityBadge.css';

export interface AgentIdentityBadgeProps {
  readonly identity: AgentIdentity;
  readonly compact?: boolean;
}

/** Composes the accepted marker with injected, Session-owned identity text. */
export function AgentIdentityBadge({ identity, compact = false }: AgentIdentityBadgeProps) {
  const role = identity.harnessRole
    .split('_')
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toLocaleUpperCase()}${part.slice(1)}`)
    .join(' ');
  return (
    <span
      className={`agent-identity-badge${compact ? ' is-compact' : ''}`}
      aria-label={`${identity.name}, ${role}`}
    >
      <AgentIdentityMarker identity={identity} />
      {!compact && (
        <span className="agent-identity-badge__text">
          <strong>{identity.name}</strong>
          <small>{role}</small>
        </span>
      )}
    </span>
  );
}
