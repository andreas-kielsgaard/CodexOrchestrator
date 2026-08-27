import type { IdentityDefinition } from '../../application/identities';

export type PresentableIdentity = Pick<IdentityDefinition, 'displayName' | 'color' | 'shape'>;

const DEFAULT_IDENTITY_COLOR = '#5f6f65';
const DARK_FOREGROUND = '#17211b';
const LIGHT_FOREGROUND = '#ffffff';

export function identityInitials(displayName: string): string {
  const initials = displayName
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => Array.from(part)[0]?.toLocaleUpperCase() ?? '')
    .join('');

  return initials || '?';
}

export function normalizedIdentityColor(color: string): string {
  const value = color.trim();
  return /^#[0-9a-f]{6}$/i.test(value) ? value.toLowerCase() : DEFAULT_IDENTITY_COLOR;
}

export function readableIdentityForeground(color: string): string {
  const background = relativeLuminance(normalizedIdentityColor(color));
  const darkContrast = contrastRatio(background, relativeLuminance(DARK_FOREGROUND));
  const lightContrast = contrastRatio(background, relativeLuminance(LIGHT_FOREGROUND));
  return darkContrast >= lightContrast ? DARK_FOREGROUND : LIGHT_FOREGROUND;
}

function contrastRatio(first: number, second: number): number {
  const lightest = Math.max(first, second);
  const darkest = Math.min(first, second);
  return (lightest + 0.05) / (darkest + 0.05);
}

function relativeLuminance(color: string): number {
  const value = color.slice(1);
  const channels = [0, 2, 4].map((index) => Number.parseInt(value.slice(index, index + 2), 16));
  const [red, green, blue] = channels.map((channel) => {
    const normalized = channel / 255;
    return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}
