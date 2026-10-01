import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessions/selections';
import type {
  SessionQueueViewDto,
  TargetSwitchRequestDto,
  UpdateQueuedMessageInputDto,
  UpdateTargetOperationInputDto,
} from '../../application/agentSessions/sessionQueue';
const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function useSessionQueue({
  client,
  queueClient,
  selectedIdRef,
  enabled,
  reloadHistory,
  setError,
}: {
  client: AgentSessionClient;
  queueClient?: AgentSessionProfileClient;
  selectedIdRef: RefObject<string | null>;
  enabled: boolean;
  reloadHistory(sessionId: string, reload?: boolean): Promise<unknown>;
  setError(error: string): void;
}) {
  const [queue, setQueueState] = useState<SessionQueueViewDto | null>(null);
  const queueRef = useRef<SessionQueueViewDto | null>(null);
  const setQueue = useCallback(
    (snapshot: SessionQueueViewDto | null) => {
      const previous = queueRef.current;
      if (
        snapshot &&
        (snapshot.sessionId !== selectedIdRef.current ||
          (previous?.sessionId === snapshot.sessionId && previous.revision > snapshot.revision))
      )
        return;
      queueRef.current = snapshot;
      setQueueState(snapshot);
    },
    [selectedIdRef],
  );
  const generation = useRef(0);
  const reloadQueue = useCallback(
    async (sessionId: string) => {
      if (!enabled || !queueClient?.loadQueue || selectedIdRef.current !== sessionId) return null;
      const request = ++generation.current;
      const snapshot = await queueClient.loadQueue(sessionId);
      if (selectedIdRef.current === sessionId && request === generation.current) setQueue(snapshot);
      return snapshot;
    },
    [enabled, queueClient, selectedIdRef, setQueue],
  );
  useEffect(() => {
    if (!enabled || !queueClient?.loadQueue) return;
    const counter = generation;
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void client
      .subscribeUpdates((update) => {
        if (update.kind === 'queue_updated' && update.sessionId === selectedIdRef.current)
          void reloadQueue(update.sessionId).catch((error) => setError(errorMessage(error)));
      })
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch((error) => setError(errorMessage(error)));
    return () => {
      disposed = true;
      counter.current++;
      unsubscribe?.();
    };
  }, [client, enabled, queueClient, reloadQueue, selectedIdRef, setError]);
  const reorderQueue = useCallback(
    async (entryIds: readonly string[]) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queue || !queueClient?.reorderQueue) return;
      try {
        setQueue(
          await queueClient.reorderQueue({
            sessionId,
            expectedRevision: queue.revision,
            entryIds,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
        await reloadHistory(sessionId, true);
      }
    },
    [reloadHistory, queueClient, queue, selectedIdRef, setError, setQueue],
  );

  const enqueueTargetOperation = useCallback(
    async (request: TargetSwitchRequestDto, beforeEntryId?: string) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queueClient?.enqueueTargetOperation) return;
      try {
        setQueue(await queueClient.enqueueTargetOperation({ sessionId, request, beforeEntryId }));
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [queueClient, selectedIdRef, setError, setQueue],
  );

  const compact = useCallback(async () => {
    const sessionId = selectedIdRef.current;
    if (!sessionId || !queueClient?.enqueueCompaction) return;
    try {
      setQueue(await queueClient.enqueueCompaction(sessionId));
      await reloadHistory(sessionId, true);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }, [reloadHistory, queueClient, selectedIdRef, setError, setQueue]);

  const updateQueuedMessage = useCallback(
    async (input: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'expectedRevision'>) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queue || !queueClient?.updateQueuedMessage) return;
      try {
        setQueue(
          await queueClient.updateQueuedMessage({
            ...input,
            sessionId,
            expectedRevision: queue.revision,
          }),
        );
        void reloadHistory(sessionId, true).catch((error) => setError(errorMessage(error)));
      } catch (caught) {
        setError(errorMessage(caught));
        throw caught;
      }
    },
    [reloadHistory, queueClient, queue, selectedIdRef, setError, setQueue],
  );

  const updateTargetOperation = useCallback(
    async (input: Omit<UpdateTargetOperationInputDto, 'sessionId' | 'expectedRevision'>) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queue || !queueClient?.updateTargetOperation) return;
      try {
        setQueue(
          await queueClient.updateTargetOperation({
            ...input,
            sessionId,
            expectedRevision: queue.revision,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [queueClient, queue, selectedIdRef, setError, setQueue],
  );

  const actOnQueueEntry = useCallback(
    async (kind: 'retry' | 'cancel', entryId: string) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queueClient) return;
      try {
        if (kind === 'retry') await queueClient.retryQueueEntry?.(sessionId, entryId);
        else await queueClient.cancelQueueEntry?.(sessionId, entryId);
        await reloadHistory(sessionId, true);
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [reloadHistory, queueClient, selectedIdRef, setError],
  );

  const sendQueueEntryNow = useCallback(
    async (
      entryId: string,
      message?: Omit<UpdateQueuedMessageInputDto, 'sessionId' | 'entryId' | 'expectedRevision'>,
    ) => {
      const sessionId = selectedIdRef.current;
      const snapshot = queueRef.current;
      if (!sessionId || snapshot?.sessionId !== sessionId || !queueClient?.sendQueueEntryNow)
        return false;
      try {
        setQueue(
          await queueClient.sendQueueEntryNow({
            sessionId,
            entryId,
            expectedRevision: snapshot.revision,
            ...(message ? { message } : {}),
          }),
        );
        void reloadHistory(sessionId, true).catch((error) => setError(errorMessage(error)));
        return true;
      } catch (caught) {
        setError(errorMessage(caught));
        void reloadQueue(sessionId).catch((error) => setError(errorMessage(error)));
        return false;
      }
    },
    [queueClient, selectedIdRef, setQueue, reloadHistory, reloadQueue, setError],
  );

  const recalculateTargetProjection = useCallback(
    async (entryId: string) => {
      const sessionId = selectedIdRef.current;
      if (!sessionId || !queue || !queueClient?.recalculateTargetProjection) return;
      try {
        setQueue(
          await queueClient.recalculateTargetProjection({
            sessionId,
            entryId,
            expectedRevision: queue.revision,
          }),
        );
      } catch (caught) {
        setError(errorMessage(caught));
      }
    },
    [queue, queueClient, selectedIdRef, setError, setQueue],
  );
  return {
    queue,
    setQueue,
    reloadQueue,
    controls: {
      enqueueTargetOperation,
      compact,
      sendQueueEntryNow: queueClient?.sendQueueEntryNow ? sendQueueEntryNow : undefined,
      reorderQueue,
      updateQueuedMessage,
      updateTargetOperation,
      recalculateTargetProjection,
      retryQueueEntry: (entryId: string) => actOnQueueEntry('retry', entryId),
      cancelQueueEntry: (entryId: string) => actOnQueueEntry('cancel', entryId),
    },
  };
}
