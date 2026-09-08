import { useCallback, useEffect, useState } from 'react';
import type {
  EventDeliveryRecordDto,
  SessionEventQueryClient,
} from '../../application/sessionEvents';

export function useSessionDeliveries(
  client: SessionEventQueryClient | undefined,
  sessionId: string | null,
) {
  const [deliveries, setDeliveries] = useState<readonly EventDeliveryRecordDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const reload = useCallback(() => setRevision((current) => current + 1), []);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void client
      ?.subscribeRecorded?.(reload)
      .then((unsubscribe) => {
        if (active) stop = unsubscribe;
        else unsubscribe();
      })
      .catch((cause) => active && setError(String(cause)));
    return () => {
      active = false;
      stop?.();
    };
  }, [client, reload]);
  useEffect(() => {
    let active = true;
    setError(null);
    setDeliveries([]);
    if (client && sessionId) {
      void client
        .listDeliveriesForSession({
          namespace: 'orchestrator.agent_sessions',
          kind: 'session',
          id: sessionId,
        })
        .then(
          (records) => active && setDeliveries(records),
          (cause) => active && setError(cause instanceof Error ? cause.message : String(cause)),
        );
    }
    return () => {
      active = false;
    };
  }, [client, sessionId, revision]);
  return { deliveries, error, reload };
}
