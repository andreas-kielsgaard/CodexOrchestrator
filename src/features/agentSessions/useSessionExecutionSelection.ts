import { useEffect, useRef, useState } from 'react';
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
  useEffect(() => {
    let active = true;
    setProfile(null);
    setError(null);
    setSelection(inherited());
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
  const afterAccepted = () => {
    if (currentSession.current === (sessionId ?? draftId))
      setSelection((current) => (current === selection ? inherited() : current));
  };
  return {
    profile,
    error,
    selection,
    setSelection,
    execution: client ? { client, selection, setSelection, afterAccepted } : undefined,
  };
}
