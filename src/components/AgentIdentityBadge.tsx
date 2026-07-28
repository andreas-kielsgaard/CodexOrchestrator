import { Bot, Compass, PackageOpen, Route, type LucideIcon } from 'lucide-react';
import type { CSSProperties } from 'react';
import type { AgentIdentityDto, AgentVisualTokenDto } from '../application/agentSessions';
import './agentIdentityBadge.css';

export interface AgentIdentityBadgeProps {
  readonly identity: AgentIdentityDto;
  readonly compact?: boolean;
}

export function AgentIdentityBadge({ identity, compact = false }: AgentIdentityBadgeProps) {
  const Icon = iconFor(identity.visualIdentity.token);
  return (
    <span
      className={`agent-identity-badge${compact ? ' is-compact' : ''}`}
      style={{ '--agent-accent': identity.visualIdentity.accent } as CSSProperties}
      aria-label={`${identity.name}, ${identity.harnessRole}`}
    >
      <span className="agent-identity-badge__marker" aria-hidden="true">
        <Icon size={compact ? 14 : 16} />
      </span>
      {!compact && (
        <span className="agent-identity-badge__text">
          <strong>{identity.name}</strong>
          <small>{identity.harnessRole}</small>
        </span>
      )}
    </span>
  );
}

function iconFor(token: AgentVisualTokenDto): LucideIcon {
  if (token === 'drafting_compass') return Compass;
  if (token === 'bootstrap_package') return PackageOpen;
  if (token === 'runner_route') return Route;
  return Bot;
}
