import { useCallback, useState } from 'react';
import type {
  BackendMaintenanceCapability,
  BackendMaintenanceResult,
} from '../../capabilities/backendMaintenance';
import { errorMessage } from '../viewModels/formatting';

export type BackendMaintenanceStatus = 'idle' | 'checking' | 'current' | 'restarting' | 'failed';

export interface BackendMaintenanceControllerState {
  status: BackendMaintenanceStatus;
  message: string;
  result: BackendMaintenanceResult | null;
  available: boolean;
}

export interface UseBackendMaintenanceControllerOptions {
  client?: BackendMaintenanceCapability;
}

export function useBackendMaintenanceController({
  client,
}: UseBackendMaintenanceControllerOptions) {
  const [state, setState] = useState<Omit<BackendMaintenanceControllerState, 'available'>>({
    status: 'idle',
    message: 'Rust backend current',
    result: null,
  });

  const checkAndReopenBackend = useCallback(() => {
    if (!client || state.status === 'checking') {
      return;
    }

    void (async () => {
      setState((current) => ({
        ...current,
        status: 'checking',
        message: 'Checking Rust backend...',
      }));

      try {
        const result = await client.checkAndReopenBackend();

        setState({
          status: result.status,
          message: result.message,
          result,
        });
      } catch (error) {
        setState({
          status: 'failed',
          message: errorMessage(error),
          result: null,
        });
      }
    })();
  }, [client, state.status]);

  return {
    state: {
      ...state,
      available: Boolean(client),
    },
    actions: {
      checkAndReopenBackend,
    },
  };
}
