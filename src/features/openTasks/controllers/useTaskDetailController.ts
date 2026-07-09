import { useCallback, useMemo, useRef, useState } from 'react';
import type {
  TaskRunDetailCapability,
  TaskRunDetailSnapshot,
} from '../../../capabilities/taskRunDetail';
import type { EntityId } from '../../../domain/model';
import { errorMessage } from '../../../app/viewModels/formatting';
import { countArtifacts } from '../viewModels/taskDetailViewModel';

export type TaskDetailStatus = 'idle' | 'loading' | 'loaded' | 'failed';

export interface TaskDetailControllerOptions {
  client: TaskRunDetailCapability;
}

export interface TaskDetailState {
  taskId: EntityId | null;
  status: TaskDetailStatus;
  snapshot: TaskRunDetailSnapshot | null;
  error: string | null;
  runCount: number;
  unlinkedArtifactCount: number;
  eventCount: number;
}

export interface TaskDetailActions {
  load(taskId: EntityId): Promise<TaskRunDetailSnapshot | null>;
  reload(): Promise<TaskRunDetailSnapshot | null>;
  close(): void;
}

export interface TaskDetailController {
  state: TaskDetailState;
  actions: TaskDetailActions;
}

const idleTaskDetailState: TaskDetailState = {
  taskId: null,
  status: 'idle',
  snapshot: null,
  error: null,
  runCount: 0,
  unlinkedArtifactCount: 0,
  eventCount: 0,
};

export function useTaskDetailController({
  client,
}: TaskDetailControllerOptions): TaskDetailController {
  const [state, setState] = useState<TaskDetailState>(idleTaskDetailState);
  const requestSequenceRef = useRef(0);
  const taskIdRef = useRef<EntityId | null>(null);

  const applySnapshot = useCallback((taskId: EntityId, snapshot: TaskRunDetailSnapshot) => {
    setState({
      taskId,
      status: 'loaded',
      snapshot,
      error: null,
      runCount: snapshot.runs.length,
      unlinkedArtifactCount: countArtifacts(snapshot.unlinkedArtifacts),
      eventCount: snapshot.eventTimeline.length,
    });
  }, []);

  const load = useCallback(
    async (taskId: EntityId): Promise<TaskRunDetailSnapshot | null> => {
      const requestSequence = requestSequenceRef.current + 1;
      requestSequenceRef.current = requestSequence;
      taskIdRef.current = taskId;

      setState((current) => ({
        ...current,
        taskId,
        status: 'loading',
        snapshot: current.taskId === taskId ? current.snapshot : null,
        error: null,
      }));

      try {
        const snapshot = await client.loadTaskRunDetail(taskId);

        if (requestSequenceRef.current === requestSequence && taskIdRef.current === taskId) {
          applySnapshot(taskId, snapshot);
        }

        return snapshot;
      } catch (caught) {
        if (requestSequenceRef.current === requestSequence && taskIdRef.current === taskId) {
          setState({
            taskId,
            status: 'failed',
            snapshot: null,
            error: errorMessage(caught),
            runCount: 0,
            unlinkedArtifactCount: 0,
            eventCount: 0,
          });
        }

        return null;
      }
    },
    [applySnapshot, client],
  );

  const close = useCallback(() => {
    requestSequenceRef.current += 1;
    taskIdRef.current = null;
    setState(idleTaskDetailState);
  }, []);

  const reload = useCallback((): Promise<TaskRunDetailSnapshot | null> => {
    return taskIdRef.current ? load(taskIdRef.current) : Promise.resolve(null);
  }, [load]);

  const actions = useMemo<TaskDetailActions>(
    () => ({
      load,
      reload,
      close,
    }),
    [close, load, reload],
  );

  return { state, actions };
}
