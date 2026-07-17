import type { AgentSessionClient } from '../../application/agentSessions';

/** Application-owned dependencies for an embedded Agent Session surface. */
export interface EmbeddedAgentSessionComposition {
  readonly client: AgentSessionClient;
  /** Writability is explicit; session presence or role never implies a composer. */
  readonly writableSessionIds?: ReadonlySet<string>;
}

export function embeddedSessionIsWritable(
  composition: EmbeddedAgentSessionComposition,
  sessionId: string,
): boolean {
  return composition.writableSessionIds?.has(sessionId) ?? false;
}
