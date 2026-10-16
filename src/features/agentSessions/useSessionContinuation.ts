import { useEffect, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
type Continuation = Awaited<ReturnType<NonNullable<AgentSessionClient['continuation']>>>;
export function useSessionContinuation(
  client: AgentSessionClient,
  sessionId: string | null | undefined,
) {
  const [current, setCurrent] = useState<{ id: string; value: Continuation }>();
  useEffect(() => {
    if (!sessionId || !client.continuation) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const value = await client.continuation!(sessionId);
        if (active) setCurrent({ id: sessionId, value });
      } finally {
        if (active) timer = setTimeout(() => void refresh().catch(() => {}), 3000);
      }
    };
    void refresh().catch(() => {});
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [client, sessionId]);
  return current && current.id === sessionId ? current.value : undefined;
}
