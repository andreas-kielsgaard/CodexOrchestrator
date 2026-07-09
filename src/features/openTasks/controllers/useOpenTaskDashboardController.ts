import { useCallback, useEffect, useState } from 'react';
import { errorMessage } from '../../../app/viewModels/formatting';
import type {
  LoadOpenTaskDashboardCapability,
  TaskDashboardSnapshot,
} from '../../../capabilities/openTaskDashboard';
import { emptyOpenTaskDashboardSnapshot } from '../../../capabilities/openTaskDashboard';

export type OpenTaskDashboardBusyAction =
  | 'load'
  | 'register-repo'
  | 'discover-repos'
  | 'create'
  | `update:${string}`
  | `archive:${string}`
  | null;

export interface UseOpenTaskDashboardControllerInput {
  taskDashboardClient: LoadOpenTaskDashboardCapability;
  onSnapshotApplied?(snapshot: TaskDashboardSnapshot): void;
  startupLoadTimeoutMs?: number;
}

export interface OpenTaskDashboardController {
  snapshot: TaskDashboardSnapshot;
  busyAction: OpenTaskDashboardBusyAction;
  error: string | null;
  hasLoadedDashboard: boolean;
  applySnapshot(snapshot: TaskDashboardSnapshot): void;
  loadDashboard(): Promise<boolean>;
  runClientAction(
    action: Exclude<OpenTaskDashboardBusyAction, null>,
    write: () => Promise<TaskDashboardSnapshot>,
  ): Promise<boolean>;
  runSideEffectAction(
    action: Exclude<OpenTaskDashboardBusyAction, null>,
    work: () => Promise<void>,
  ): Promise<boolean>;
  setError(error: string | null): void;
}

export function useOpenTaskDashboardController({
  taskDashboardClient,
  onSnapshotApplied,
  startupLoadTimeoutMs = defaultStartupLoadTimeoutMs,
}: UseOpenTaskDashboardControllerInput): OpenTaskDashboardController {
  const [snapshot, setSnapshot] = useState<TaskDashboardSnapshot>(() =>
    emptyOpenTaskDashboardSnapshot(),
  );
  const [busyAction, setBusyAction] = useState<OpenTaskDashboardBusyAction>('load');
  const [error, setError] = useState<string | null>(null);
  const [hasLoadedDashboard, setHasLoadedDashboard] = useState(false);

  const applySnapshot = useCallback(
    (nextSnapshot: TaskDashboardSnapshot) => {
      setSnapshot(nextSnapshot);
      onSnapshotApplied?.(nextSnapshot);
    },
    [onSnapshotApplied],
  );

  const runClientAction = useCallback(
    async (
      action: Exclude<OpenTaskDashboardBusyAction, null>,
      write: () => Promise<TaskDashboardSnapshot>,
    ) => {
      setBusyAction(action);
      setError(null);

      try {
        applySnapshot(await write());
        return true;
      } catch (caught) {
        setError(errorMessage(caught));
        return false;
      } finally {
        setBusyAction(null);
      }
    },
    [applySnapshot],
  );

  const runSideEffectAction = useCallback(
    async (action: Exclude<OpenTaskDashboardBusyAction, null>, work: () => Promise<void>) => {
      setBusyAction(action);
      setError(null);

      try {
        await work();
        return true;
      } catch (caught) {
        setError(errorMessage(caught));
        return false;
      } finally {
        setBusyAction(null);
      }
    },
    [],
  );

  const loadDashboard = useCallback(async () => {
    if (
      await runClientAction('load', () =>
        withTimeout(
          taskDashboardClient.loadDashboard(),
          startupLoadTimeoutMs,
          'Dashboard backend did not respond during startup. It may still be starting; retry once the Tauri window settles.',
        ),
      )
    ) {
      setHasLoadedDashboard(true);
      return true;
    }

    return false;
  }, [runClientAction, startupLoadTimeoutMs, taskDashboardClient]);

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  return {
    snapshot,
    busyAction,
    error,
    hasLoadedDashboard,
    applySnapshot,
    loadDashboard,
    runClientAction,
    runSideEffectAction,
    setError,
  };
}

const defaultStartupLoadTimeoutMs = 10_000;

function withTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timeoutId = window.setTimeout(() => reject(new Error(message)), timeoutMs);

    promise.then(
      (value) => {
        window.clearTimeout(timeoutId);
        resolve(value);
      },
      (error: unknown) => {
        window.clearTimeout(timeoutId);
        reject(error);
      },
    );
  });
}
