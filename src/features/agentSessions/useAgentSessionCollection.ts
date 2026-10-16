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
const collectionSnapshots = new WeakMap<object, ReturnType<typeof emptySessionNavigation>>();
export function useAgentSessionCollection(
  client: AgentSessionClient,
  navigation?: SessionNavigationClient,
) {
  const owner = navigation ?? client;
  const [data, setData] = useState(
    () => collectionSnapshots.get(owner) ?? emptySessionNavigation(),
  );
  const [loading, setLoading] = useState(() => !collectionSnapshots.has(owner));
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const refresh = useCallback(
    async (showLoading = true) => {
      const request = ++generation.current;
      if (showLoading && !collectionSnapshots.has(owner)) {
        setLoading(true);
        setError(null);
      }
      try {
        const next = navigation
          ? await Promise.all([navigation.load(), client.listSessions({ availability: 'available' })]).then(([navigation, summaries]) => ({
              ...navigation,
              summaries: [
                ...navigation.summaries.map((entry) => summaries.find((summary) => summary.id === entry.id && summary.updatedAt > entry.updatedAt) ?? entry),
                ...summaries.filter((summary) => !navigation.summaries.some((entry) => entry.id === summary.id)),
              ],
            }))
          : {
              ...emptySessionNavigation(),
              summaries: await client.listSessions({ availability: 'available' }),
            };
        if (request === generation.current) {
          collectionSnapshots.set(owner, next);
          setData(next);
        }
      } catch (caught) {
        if (request === generation.current)
          setError(`Session list reload failed: ${sessionErrorMessage(caught)}`);
      } finally {
        if (request === generation.current) setLoading(false);
      }
    },
    [client, navigation, owner],
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
    const inventoryRefresh = navigation ? setInterval(() => void refresh(false), 60000) : undefined;
    return () => {
      active = false;
      invalidatePending();
      clearTimeout(timer);
      clearInterval(inventoryRefresh);
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
  const rename = useCallback(
    async (id: string, title: string) => {
      try {
        if (!client.renameSession) throw new Error('Session renaming is unavailable.');
        await client.renameSession({ sessionId: id, title });
        await refresh(false);
      } catch (cause) {
        setError(sessionErrorMessage(cause));
        throw cause;
      }
    },
    [client, refresh],
  );
  const clearError = useCallback(() => setError(null), []);
  const markAttention = useCallback(
    async (id: string, unread: boolean) => {
      if (!navigation?.markAttention) return;
      setData((current) => {
        const next = {
          ...current,
          summaries: current.summaries.map((s) =>
            s.id === id
              ? {
                  ...s,
                  completionState: unread
                    ? ((s.hasActiveInvocation ? 'continued' : 'done') as 'continued' | 'done')
                    : null,
                }
              : s,
          ),
        };
        collectionSnapshots.set(owner, next);
        return next;
      });
      await mutate(() => navigation.markAttention!(id, unread));
    },
    [navigation, owner, mutate],
  );
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
    rename,
    markAttention,
  };
}
