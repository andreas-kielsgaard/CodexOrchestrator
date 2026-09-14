import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  SessionNavigationAgentAccess,
  SessionNavigationCommandRequest,
  SessionNavigationState,
} from '../application/agentSessions/agentAccess';

export function useSessionNavigationAgentConnection(
  source: SessionNavigationAgentAccess | undefined,
  activate: () => void,
) {
  const [request, setRequest] = useState<SessionNavigationCommandRequest | null>(null);
  const activateRef = useRef(activate);
  activateRef.current = activate;
  useEffect(() => {
    if (!source) return;
    let active = true;
    let stop: (() => void) | undefined;
    void source
      .subscribe((value) => {
        if (active) {
          activateRef.current();
          setRequest(value);
        }
      })
      .then(
        (value) => {
          if (active) stop = value;
          else value();
        },
        (error) => console.error('Session navigation agent connection failed', error),
      );
    return () => {
      active = false;
      stop?.();
    };
  }, [source]);
  const complete = useCallback(
    async (id: string, state: SessionNavigationState | null, error: string | null) => {
      await source?.complete(id, state, error);
      setRequest((current) => (current?.id === id ? null : current));
    },
    [source],
  );
  return { request, complete };
}
