import { AgentIdentityMarker } from './AgentIdentityMarker';
import type { PresentableIdentity } from './identityPresentation';
import './identities.css';

export interface AgentIdentityBadgeProps {
  readonly identity: PresentableIdentity;
  readonly secondaryLabel?: string;
  readonly compact?: boolean;
}

/** Identity presentation with optional context supplied by its caller. */
export function AgentIdentityBadge({
  identity,
  secondaryLabel,
  compact = false,
}: AgentIdentityBadgeProps) {
  const accessibleLabel = secondaryLabel
    ? `${identity.displayName}, ${secondaryLabel}`
    : identity.displayName;

  return (
    <span className={`identity-badge${compact ? ' is-compact' : ''}`} aria-label={accessibleLabel}>
      <AgentIdentityMarker identity={identity} />
      {!compact && (
        <span className="identity-badge__text" aria-hidden="true">
          <strong>{identity.displayName}</strong>
          {secondaryLabel && <small>{secondaryLabel}</small>}
        </span>
      )}
    </span>
  );
}
