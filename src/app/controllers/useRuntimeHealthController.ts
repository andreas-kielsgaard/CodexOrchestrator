import { useCallback, useEffect, useMemo, useState } from 'react';
import type {
  RuntimeHealthCapability,
  RuntimeStatusSnapshot,
} from '../../capabilities/runtimeHealth';
import { errorMessage } from '../viewModels/formatting';
import { formatStaleTargets } from '../viewModels/runtimeStatusViewModel';

export type RuntimeHealthStatus = 'idle' | 'checking' | 'ready' | 'failed';

export interface RuntimeHealthControllerOptions {
  client?: RuntimeHealthCapability;
  pollIntervalMs?: number;
  autoStart?: boolean;
}

export interface RuntimeHealthState {
  status: RuntimeHealthStatus;
  snapshot: RuntimeStatusSnapshot | null;
  error: string | null;
  staleGeneration: string;
  staleNoticeVisible: boolean;
  staleNoticeMessage: string | null;
}

export interface RuntimeHealthActions {
  check(): Promise<RuntimeStatusSnapshot | null>;
  clearStale(): Promise<RuntimeStatusSnapshot | null>;
  dismissStaleNotice(): void;
  resetDismissedStaleNotice(): void;
}

export interface RuntimeHealthController {
  state: RuntimeHealthState;
  actions: RuntimeHealthActions;
}

const defaultPollIntervalMs = 5000;
const fallbackStaleGeneration = 'stale-runtime';

export function useRuntimeHealthController({
  client,
  pollIntervalMs = defaultPollIntervalMs,
  autoStart = true,
}: RuntimeHealthControllerOptions): RuntimeHealthController {
  const [status, setStatus] = useState<RuntimeHealthStatus>(() =>
    client && autoStart ? 'checking' : 'idle',
  );
  const [snapshot, setSnapshot] = useState<RuntimeStatusSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dismissedStaleGeneration, setDismissedStaleGeneration] = useState<string | null>(null);

  const check = useCallback(async (): Promise<RuntimeStatusSnapshot | null> => {
    if (!client) {
      setStatus('idle');
      setSnapshot(null);
      setError(null);
      return null;
    }

    setStatus('checking');
    setError(null);

    try {
      const nextSnapshot = await client.checkStatus();
      setSnapshot(nextSnapshot);
      setStatus('ready');
      return nextSnapshot;
    } catch (caught) {
      setError(errorMessage(caught));
      setStatus('failed');
      return null;
    }
  }, [client]);

  const clearStale = useCallback(async (): Promise<RuntimeStatusSnapshot | null> => {
    if (!client?.clearStale) {
      return snapshot;
    }

    try {
      const nextSnapshot = await client.clearStale();
      setSnapshot(nextSnapshot);
      return nextSnapshot;
    } catch (caught) {
      setError(errorMessage(caught));
      return null;
    }
  }, [client, snapshot]);

  useEffect(() => {
    if (!client || !autoStart) {
      return;
    }

    let cancelled = false;

    const checkIfCurrent = async () => {
      if (!cancelled) {
        await check();
      }
    };

    void checkIfCurrent();
    const intervalId = window.setInterval(() => {
      void checkIfCurrent();
    }, pollIntervalMs);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [autoStart, check, client, pollIntervalMs]);

  const staleGeneration = snapshot?.generation ?? fallbackStaleGeneration;
  const staleNoticeVisible =
    snapshot?.stale === true && dismissedStaleGeneration !== staleGeneration;
  const staleNoticeMessage =
    snapshot?.stale === true
      ? `${formatStaleTargets(snapshot.staleTargets)} changed${
          snapshot.reason ? `: ${snapshot.reason}` : ''
        }.`
      : null;

  const state = useMemo<RuntimeHealthState>(
    () => ({
      status,
      snapshot,
      error,
      staleGeneration,
      staleNoticeVisible,
      staleNoticeMessage,
    }),
    [error, snapshot, staleGeneration, staleNoticeMessage, staleNoticeVisible, status],
  );

  const actions = useMemo<RuntimeHealthActions>(
    () => ({
      check,
      clearStale,
      dismissStaleNotice: () => setDismissedStaleGeneration(staleGeneration),
      resetDismissedStaleNotice: () => setDismissedStaleGeneration(null),
    }),
    [check, clearStale, staleGeneration],
  );

  return { state, actions };
}
