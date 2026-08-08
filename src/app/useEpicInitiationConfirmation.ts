import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  EpicInitiationConfirmationError,
  confirmationErrorMessage,
  type EpicInitiationConfirmationClient,
  type EpicInitiationConfirmationDetails,
  type EpicInitiationConfirmationEvent,
  type EpicInitiationConfirmationRequest,
} from '../application/orchestrations';

interface QueuedConfirmation {
  readonly request: EpicInitiationConfirmationRequest;
  readonly details?: EpicInitiationConfirmationDetails;
  readonly detailsUnavailable?: boolean;
}

export function useEpicInitiationConfirmation(
  client?: EpicInitiationConfirmationClient,
  onProjected?: () => Promise<void>,
) {
  const [queue, setQueue] = useState<readonly QueuedConfirmation[]>([]);
  const [resolving, setResolving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [receiptError, setReceiptError] = useState<string | null>(null);
  const queueRef = useRef(queue);
  queueRef.current = queue;

  const ingest = useCallback(
    (request: EpicInitiationConfirmationRequest) => {
      setQueue((current) =>
        current.some((item) => item.request.requestId === request.requestId)
          ? current
          : [...current, { request }],
      );
      if (!client) return;
      void client.describe(request).then(
        (details) =>
          setQueue((current) =>
            current.map((item) =>
              item.request.requestId === request.requestId ? { ...item, details } : item,
            ),
          ),
        () =>
          setQueue((current) =>
            current.map((item) =>
              item.request.requestId === request.requestId
                ? { ...item, detailsUnavailable: true }
                : item,
            ),
          ),
      );
    },
    [client],
  );

  useEffect(() => {
    if (!client) return;
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void client
      .subscribe(
        (event: EpicInitiationConfirmationEvent) => {
          if (active && event.state === 'requested') ingest(event.request);
        },
        () => active && setReceiptError(confirmationErrorMessage('malformed_event')),
      )
      .then((next) => {
        if (active) unsubscribe = next;
        else next();
      })
      .catch(() => active && setReceiptError(confirmationErrorMessage('unavailable')));
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [client, ingest]);

  const requestButton = useCallback(
    async (input: {
      epicPlanningDraftId: string;
      expectedRevisionToken: string;
      idempotencyKey: string;
    }) => {
      if (!client) throw new EpicInitiationConfirmationError('unavailable');
      setReceiptError(null);
      try {
        const request = await client.request(input);
        ingest(request);
      } catch (failure) {
        const kind =
          failure instanceof EpicInitiationConfirmationError ? failure.kind : 'unavailable';
        setReceiptError(confirmationErrorMessage(kind));
        throw failure;
      }
    },
    [client, ingest],
  );

  const resolve = useCallback(
    async (decision: 'confirmed' | 'rejected', rootBranch?: string) => {
      const current = queueRef.current[0];
      if (!client || !current || resolving) return false;
      setResolving(true);
      setError(null);
      try {
        if (decision === 'confirmed') {
          await client.resolve(current.request.requestId, decision, rootBranch);
        } else {
          await client.resolve(current.request.requestId, decision);
        }
      } catch (failure) {
        const kind =
          failure instanceof EpicInitiationConfirmationError ? failure.kind : 'unavailable';
        setResolving(false);
        if (decision === 'rejected' && kind === 'rejected') {
          setQueue((items) => items.filter((item) => item !== current));
          return false;
        }
        setError(confirmationErrorMessage(kind));
        return false;
      }
      setQueue((items) => items.filter((item) => item !== current));
      setResolving(false);
      if (decision !== 'confirmed') return false;
      try {
        await onProjected?.();
      } catch {
        setReceiptError(
          'Epic initiation was confirmed, but current application state could not be refreshed. Orchestration data is unavailable until refresh succeeds.',
        );
      }
      return true;
    },
    [client, onProjected, resolving],
  );

  return useMemo(
    () => ({
      current: queue[0],
      queuedCount: Math.max(0, queue.length - 1),
      resolving,
      error,
      receiptError,
      requestButton,
      resolve,
    }),
    [error, queue, receiptError, requestButton, resolve, resolving],
  );
}
