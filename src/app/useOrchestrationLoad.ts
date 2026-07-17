import { useCallback, useEffect, useState } from 'react';
import type {
  OrchestrationApplicationClient,
  OrchestrationLoadResult,
} from '../application/orchestrations/orchestrationClient';

export type OrchestrationLoadState = ({ readonly kind: 'loading' } | OrchestrationLoadResult) & {
  refresh(): Promise<boolean>;
};

export function useOrchestrationLoad(
  client: OrchestrationApplicationClient,
): OrchestrationLoadState {
  const [state, setState] = useState<{ readonly kind: 'loading' } | OrchestrationLoadResult>({
    kind: 'loading',
  });
  const refresh = useCallback(async () => {
    setState({ kind: 'loading' });
    try {
      const next = await client.load();
      setState(next);
      return next.kind !== 'unavailable' && next.kind !== 'failed';
    } catch {
      setState({ kind: 'failed', message: 'Orchestration data could not be loaded.' });
      return false;
    }
  }, [client]);

  useEffect(() => {
    let active = true;
    void (async () => {
      setState({ kind: 'loading' });
      try {
        const result = await client.load();
        if (active) setState(result);
      } catch {
        if (active)
          setState({ kind: 'failed', message: 'Orchestration data could not be loaded.' });
      }
    })();
    return () => {
      active = false;
    };
  }, [client]);

  return { ...state, refresh };
}
