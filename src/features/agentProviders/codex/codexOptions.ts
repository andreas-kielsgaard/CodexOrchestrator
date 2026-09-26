import type { ProviderNativeOptionsDto } from '../../../application/agentProviders';

export const CODEX_PROVIDER = 'codex';
export type CodexPersonality = 'none' | 'friendly' | 'pragmatic';

const PERSONALITIES: readonly CodexPersonality[] = ['none', 'friendly', 'pragmatic'];

/** Reads the Codex personality override from a route's native options. */
export function codexPersonality(
  options: ProviderNativeOptionsDto | null | undefined,
): CodexPersonality | null {
  if (options?.provider !== CODEX_PROVIDER) return null;
  const settings = options.settings as { personality?: unknown } | null;
  const personality = settings?.personality;
  return PERSONALITIES.find((value) => value === personality) ?? null;
}

/** Inherit is an absent envelope, never an empty one. */
export function codexOptions(personality: CodexPersonality | null): ProviderNativeOptionsDto | null {
  return personality ? { provider: CODEX_PROVIDER, settings: { personality } } : null;
}
