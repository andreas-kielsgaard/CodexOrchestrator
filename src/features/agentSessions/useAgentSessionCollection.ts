import { useCallback, useEffect, useRef, useState } from 'react';
import type { AgentSessionClient, AgentSessionSummaryDto } from '../../application/agentSessions';
import type { AgentSessionCollectionController } from './useAgentSessionController';
import { sessionErrorMessage as errorMessage } from './sessionErrors';
import { sessionSummaryChanged } from './sessionAttention';
export interface AgentSessionCollectionOptions {
  readonly selectedSessionId?: string | null;
  readonly onSelectedSessionChange?: (sessionId: string | null) => void;
}

export function useAgentSessionCollection(
  client: AgentSessionClient,
  options: AgentSessionCollectionOptions = {},
): AgentSessionCollectionController {
  const { selectedSessionId: controlledSelectedSessionId, onSelectedSessionChange } = options;
  const [summaries, setSummaries] = useState<AgentSessionSummaryDto[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(
    controlledSelectedSessionId ?? null,
  );
  const selectedSessionIdRef = useRef<string | null>(controlledSelectedSessionId ?? null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generationRef = useRef(0);
  const refresh = useCallback(
    async (showLoading: boolean) => {
      const generation = ++generationRef.current;
      if (showLoading) {
        setLoading(true);
        setError(null);
      }
      try {
        const next = await client.listSessions({ availability: 'available' });
        if (!mountedRef.current || generation !== generationRef.current) return;
        setSummaries(next);
        if (!showLoading) return;
        const selected =
          selectedSessionIdRef.current ?? controlledSelectedSessionId ?? next[0]?.id ?? null;
        if (selected !== selectedSessionIdRef.current) {
          selectedSessionIdRef.current = selected;
          setSelectedSessionId(selected);
          onSelectedSessionChange?.(selected);
        }
      } catch (caught) {
        if (mountedRef.current && generation === generationRef.current)
          setError(`Session list reload failed: ${errorMessage(caught)}`);
      } finally {
        if (mountedRef.current && generation === generationRef.current) setLoading(false);
      }
    },
    [client, controlledSelectedSessionId, onSelectedSessionChange],
  );
  const reload = useCallback(() => refresh(true), [refresh]);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    void reload();
    return () => {
      mountedRef.current = false;
    };
  }, [reload]);
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let unsubscribe: (() => void) | undefined;
    void client
      .subscribeUpdates((update) => {
        if (disposed || !sessionSummaryChanged(update)) return;
        clearTimeout(timer);
        timer = setTimeout(() => void refresh(false), 100);
      })
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch((caught) => {
        if (!disposed) setError(`Session list updates failed: ${errorMessage(caught)}`);
      });
    return () => {
      disposed = true;
      clearTimeout(timer);
      unsubscribe?.();
    };
  }, [client, refresh]);
  useEffect(() => {
    if (controlledSelectedSessionId === undefined) return;
    selectedSessionIdRef.current = controlledSelectedSessionId;
    setSelectedSessionId(controlledSelectedSessionId);
  }, [controlledSelectedSessionId]);
  return {
    summaries,
    selectedSessionId,
    loading,
    error,
    selectSession: async (id) => {
      setError(null);
      selectedSessionIdRef.current = id;
      setSelectedSessionId(id);
      onSelectedSessionChange?.(id);
    },
    startNewSession: () => {
      setError(null);
      selectedSessionIdRef.current = null;
      setSelectedSessionId(null);
      onSelectedSessionChange?.(null);
    },
    reload,
    clearError: () => setError(null),
  };
}
