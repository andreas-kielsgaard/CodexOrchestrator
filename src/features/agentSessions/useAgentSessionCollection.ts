import { useCallback, useEffect, useRef, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import {
  emptySessionNavigation,
  type SessionNavigationClient,
  type SessionPlacement,
} from '../../application/agentSessions/organization';
import type { NavigationOrderScope } from '../../application/agentSessions/navigationOrder';
import { sessionErrorMessage } from './sessionErrors';
import { sessionSummaryChanged } from './sessionAttention';
export function useAgentSessionCollection(
  client: AgentSessionClient,
  navigation?: SessionNavigationClient,
) {
  const [data, setData] = useState(emptySessionNavigation);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const refresh = useCallback(
    async (showLoading = true) => {
      const request = ++generation.current;
      if (showLoading) {
        setLoading(true);
        setError(null);
      }
      try {
        const next = navigation
          ? await navigation.load()
          : {
              ...emptySessionNavigation(),
              summaries: await client.listSessions({ availability: 'available' }),
            };
        if (request === generation.current) setData(next);
      } catch (caught) {
        if (request === generation.current)
          setError(`Session list reload failed: ${sessionErrorMessage(caught)}`);
      } finally {
        if (request === generation.current) setLoading(false);
      }
    },
    [client, navigation],
  );
  const reload = useCallback(() => refresh(true), [refresh]);
  const invalidatePending = useCallback(() => {
    generation.current++;
  }, []);
  useEffect(() => {
    void reload();
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const stops: (() => void)[] = [];
    const changed = () => {
      if (!active) return;
      clearTimeout(timer);
      timer = setTimeout(() => void refresh(false), 100);
    };
    const retain = (stop: () => void) => {
      if (active) stops.push(stop);
      else stop();
    };
    const failed = (cause: unknown) => {
      if (active) setError(sessionErrorMessage(cause));
    };
    void client
      .subscribeUpdates((update) => {
        if (sessionSummaryChanged(update)) changed();
      })
      .then(retain, failed);
    void navigation?.subscribeChanged?.(changed).then(retain, failed);
    return () => {
      active = false;
      invalidatePending();
      clearTimeout(timer);
      stops.forEach((stop) => stop());
    };
  }, [client, navigation, reload, refresh, invalidatePending]);
  const mutate = useCallback(
    async (operation: () => Promise<void>) => {
      try {
        await operation();
        await refresh(false);
      } catch (cause) {
        setError(sessionErrorMessage(cause));
      }
    },
    [refresh],
  );
  const move = useCallback(
    (id: string, placement: SessionPlacement, orderedIds?: readonly string[]) =>
      navigation ? mutate(() => navigation.move(id, placement, orderedIds)) : Promise.resolve(),
    [mutate, navigation],
  );
  const reorder = useCallback(
    (scope: NavigationOrderScope, ids: readonly string[]) =>
      navigation ? mutate(() => navigation.reorder(scope, ids)) : Promise.resolve(),
    [mutate, navigation],
  );
  const pin = useCallback(
    (id: string, pinned: boolean) =>
      navigation ? mutate(() => navigation.pin(id, pinned)) : Promise.resolve(),
    [mutate, navigation],
  );
  const clearError = useCallback(() => setError(null), []);
  return {
    data,
    summaries: data.summaries,
    loading,
    error,
    reload,
    clearError,
    move,
    reorder,
    pin,
  };
}
