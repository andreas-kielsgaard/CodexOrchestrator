import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
} from '../../application/agentSessions';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

const inherited = (): PerMessageRuntimeSelection => ({ model: null, reasoningMode: null });

/** Shared configuration read and message-local selection state for every profiled Session view. */
export function useSessionExecutionSelection(
  client: AgentSessionProfileClient | undefined,
  sessionId: string | null,
  draftId?: string,
) {
  const [profile, setProfile] = useState<PinnedAgentSessionProfileDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState(inherited);
  const currentSession = useRef(sessionId ?? draftId);
  currentSession.current = sessionId ?? draftId;
  const previousContext = useRef({ sessionId, draftId });
  useEffect(() => {
    let active = true;
    setProfile(null);
    setError(null);
    if (previousContext.current.sessionId || !sessionId) setSelection(inherited());
    previousContext.current = { sessionId, draftId };
    if (client && sessionId)
      void client.loadPinnedProfile(sessionId).then(
        (value) => {
          if (active) setProfile(value);
        },
        (cause) => {
          if (active) setError(cause instanceof Error ? cause.message : String(cause));
        },
      );
    return () => {
      active = false;
    };
  }, [client, sessionId, draftId]);
  const reloadProfile = useCallback(async () => {
    if (!client || !sessionId) return;
    try {
      const value = await client.loadPinnedProfile(sessionId);
      if (currentSession.current === sessionId) {
        setProfile(value);
        setError(null);
      }
    } catch (cause) {
      if (currentSession.current === sessionId)
        setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [client, sessionId]);
  const afterAccepted = () => {
    if (currentSession.current === (sessionId ?? draftId))
      setSelection((current) => (current === selection ? inherited() : current));
  };
  return {
    profile,
    error,
    reloadProfile,
    selection,
    setSelection,
    execution: client ? { client, selection, setSelection, afterAccepted } : undefined,
  };
}
