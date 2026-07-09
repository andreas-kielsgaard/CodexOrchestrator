import { useCallback, useMemo, useRef, useState } from 'react';
import type {
  LoadOpenTaskDashboardCapability,
  TaskDashboardSnapshot,
} from '../../../capabilities/openTaskDashboard';
import type { TaskRunLaunchCapability } from '../../../capabilities/taskRunLaunch';
import type { EntityId } from '../../../domain/model';
import { errorMessage } from '../../../app/viewModels/formatting';
import { formatRunResult } from '../viewModels/runResultViewModel';

export type TaskRunActionStatus = 'running' | 'completed' | 'failed';

export interface TaskRunActionState {
  status: TaskRunActionStatus;
  message: string;
}

export interface TaskRunControllerState {
  prompts: Record<EntityId, string>;
  actionsByTaskId: Record<EntityId, TaskRunActionState>;
  error: string | null;
}

export interface TaskRunControllerActions {
  updatePrompt(taskId: EntityId, prompt: string): void;
  startRun(task: TaskRunStartInput): void;
  clearError(): void;
}

export interface TaskRunController {
  state: TaskRunControllerState;
  actions: TaskRunControllerActions;
}

export interface TaskRunStartInput {
  id: EntityId;
  title: string;
  summary: string;
  worktreePath?: string;
}

export interface UseTaskRunControllerInput {
  runtimeCommandClient: TaskRunLaunchCapability;
  dashboardClient: LoadOpenTaskDashboardCapability;
  selectedDetailTaskId: EntityId | null;
  onSnapshot(snapshot: TaskDashboardSnapshot): void;
  onLoadTaskDetail(taskId: EntityId): Promise<unknown>;
}

export function useTaskRunController({
  runtimeCommandClient,
  dashboardClient,
  selectedDetailTaskId,
  onSnapshot,
  onLoadTaskDetail,
}: UseTaskRunControllerInput): TaskRunController {
  const [prompts, setPrompts] = useState<Record<EntityId, string>>({});
  const [actionsByTaskId, setActionsByTaskId] = useState<Record<EntityId, TaskRunActionState>>({});
  const [error, setError] = useState<string | null>(null);
  const selectedDetailTaskIdRef = useRef<EntityId | null>(selectedDetailTaskId);

  selectedDetailTaskIdRef.current = selectedDetailTaskId;

  const updatePrompt = useCallback((taskId: EntityId, prompt: string) => {
    setPrompts((current) => ({ ...current, [taskId]: prompt }));
  }, []);

  const startRun = useCallback(
    (task: TaskRunStartInput) => {
      const prompt = (prompts[task.id] ?? '').trim();

      if (!task.worktreePath || !prompt) {
        return;
      }

      void (async () => {
        setActionsByTaskId((current) => ({
          ...current,
          [task.id]: { status: 'running', message: 'Starting Codex run...' },
        }));
        setError(null);

        try {
          const result = await runtimeCommandClient.startCodexTaskRun({
            taskId: task.id,
            prompt,
            cwd: task.worktreePath,
            conversationTitle: task.title,
            conversationSummary: task.summary,
          });

          setActionsByTaskId((current) => ({
            ...current,
            [task.id]: {
              status: result.status,
              message: formatRunResult(result),
            },
          }));

          if (result.status === 'completed') {
            setPrompts((current) => ({ ...current, [task.id]: '' }));
          }
        } catch (caught) {
          setActionsByTaskId((current) => ({
            ...current,
            [task.id]: {
              status: 'failed',
              message: `Run failed: ${errorMessage(caught)}`,
            },
          }));
        } finally {
          try {
            onSnapshot(await dashboardClient.loadDashboard());
            if (selectedDetailTaskIdRef.current === task.id) {
              await onLoadTaskDetail(task.id);
            }
          } catch (caught) {
            setError(`Dashboard reload failed: ${errorMessage(caught)}`);
          }
        }
      })();
    },
    [dashboardClient, onLoadTaskDetail, onSnapshot, prompts, runtimeCommandClient],
  );

  const actions = useMemo<TaskRunControllerActions>(
    () => ({
      updatePrompt,
      startRun,
      clearError: () => setError(null),
    }),
    [startRun, updatePrompt],
  );

  return {
    state: {
      prompts,
      actionsByTaskId,
      error,
    },
    actions,
  };
}
