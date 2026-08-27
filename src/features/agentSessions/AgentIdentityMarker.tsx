import type { AgentIdentity } from '../../application/agentSessions';
import type { CSSProperties } from 'react';
import './agentIdentityMarker.css';

export interface AgentIdentityMarkerProps {
  readonly identity: AgentIdentity;
}

/** Presentation-only marker for a session-owned identity. */
export function AgentIdentityMarker({ identity }: AgentIdentityMarkerProps) {
  const initials = identity.name
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part.charAt(0).toLocaleUpperCase())
    .join('');
  const accent = identity.visualIdentityAccent;
  const foreground = accent ? readableTextColor(accent) : undefined;
  return (
    <span
      className={`agent-identity-marker is-${identity.visualIdentityShape ?? 'circle'}`}
      data-harness-role={identity.harnessRole}
      data-visual-identity-token={identity.visualIdentityToken}
      style={
        accent
          ? ({
              '--agent-identity-accent': accent,
              '--agent-identity-foreground': foreground,
            } as CSSProperties)
          : undefined
      }
      aria-hidden="true"
    >
      {initials}
    </span>
  );
}

function readableTextColor(hex: string): '#17211b' | '#ffffff' {
  const value = hex.replace('#', '');
  if (!/^[0-9a-f]{6}$/i.test(value)) return '#17211b';
  const [red, green, blue] = [0, 2, 4].map((index) =>
    Number.parseInt(value.slice(index, index + 2), 16),
  );
  const luminance = (0.299 * red + 0.587 * green + 0.114 * blue) / 255;
  return luminance > 0.58 ? '#17211b' : '#ffffff';
}
