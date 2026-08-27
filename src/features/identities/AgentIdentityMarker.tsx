import type { CSSProperties } from 'react';
import {
  identityInitials,
  normalizedIdentityColor,
  readableIdentityForeground,
  type PresentableIdentity,
} from './identityPresentation';
import './identities.css';

export interface AgentIdentityMarkerProps {
  readonly identity: PresentableIdentity;
}

/** Decorative initials marker for an identity rendered by its parent. */
export function AgentIdentityMarker({ identity }: AgentIdentityMarkerProps) {
  const color = normalizedIdentityColor(identity.color);
  return (
    <span
      className={`identity-marker is-${identity.shape}`}
      style={
        {
          '--identity-color': color,
          '--identity-foreground': readableIdentityForeground(color),
        } as CSSProperties
      }
      aria-hidden="true"
    >
      {identityInitials(identity.displayName)}
    </span>
  );
}
