import { useCallback, useMemo, useState } from 'react';
import type {
  ArchiveOpenTaskCapability,
  TaskDashboardSnapshot,
  UpdateOpenTaskCapability,
} from '../../../capabilities/openTaskDashboard';
import type { AttentionState, EntityId, ExecutionState, Task } from '../../../domain/model';
import type { OpenTaskDashboardBusyAction } from './useOpenTaskDashboardController';

export interface TaskEditDraft {
  projectId: EntityId;
  worktreeId: EntityId;
  title: string;
  summary: string;
  attentionState: AttentionState;
  executionState: ExecutionState;
  priority: Task['priority'];
}

export interface TaskEditControllerState {
  editingTaskId: EntityId | null;
  draft: TaskEditDraft;
}

export interface TaskEditControllerActions {
  setDraft(draft: TaskEditDraft): void;
  patchDraft(patch: Partial<TaskEditDraft>): void;
  start(task: TaskEditStartInput): void;
  cancel(): void;
  save(taskId: EntityId): Promise<boolean>;
  updateState(
    taskId: EntityId,
    input: { attentionState?: AttentionState; executionState?: ExecutionState },
  ): Promise<boolean>;
  archive(taskId: EntityId): Promise<boolean>;
}

export interface TaskEditController {
  state: TaskEditControllerState;
  actions: TaskEditControllerActions;
}

export interface TaskEditStartInput {
  id: EntityId;
  title: string;
  summary: string;
  attentionState: AttentionState;
  executionState: ExecutionState;
  priority: Task['priority'];
}

export interface UseTaskEditControllerInput {
  client: UpdateOpenTaskCapability & ArchiveOpenTaskCapability;
  runDashboardAction(
    action: Exclude<OpenTaskDashboardBusyAction, null>,
    write: () => Promise<TaskDashboardSnapshot>,
  ): Promise<boolean>;
  initialDraft?: Partial<TaskEditDraft>;
  onArchived?(taskId: EntityId): void;
}

export const initialTaskEditDraft: TaskEditDraft = {
  projectId: '',
  worktreeId: '',
  title: '',
  summary: '',
  attentionState: 'needs_action_now',
  executionState: 'draft',
  priority: 'normal',
};

export function useTaskEditController({
  client,
  runDashboardAction,
  initialDraft,
  onArchived,
}: UseTaskEditControllerInput): TaskEditController {
  const [editingTaskId, setEditingTaskId] = useState<EntityId | null>(null);
  const [draft, setDraft] = useState<TaskEditDraft>(() => ({
    ...initialTaskEditDraft,
    ...initialDraft,
  }));

  const patchDraft = useCallback((patch: Partial<TaskEditDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
  }, []);

  const cancel = useCallback(() => {
    setEditingTaskId(null);
    setDraft({ ...initialTaskEditDraft, ...initialDraft });
  }, [initialDraft]);

  const start = useCallback((task: TaskEditStartInput) => {
    setEditingTaskId(task.id);
    setDraft({
      projectId: '',
      worktreeId: '',
      title: task.title,
      summary: task.summary,
      attentionState: task.attentionState,
      executionState: task.executionState,
      priority: task.priority,
    });
  }, []);

  const save = useCallback(
    async (taskId: EntityId): Promise<boolean> => {
      const title = draft.title.trim();
      const summary = draft.summary.trim();

      if (!title || !summary) {
        return false;
      }

      const saved = await runDashboardAction(`update:${taskId}`, () =>
        client.updateTask(taskId, {
          title,
          summary,
          attentionState: draft.attentionState,
          executionState: draft.executionState,
          priority: draft.priority,
        }),
      );

      if (saved) {
        cancel();
      }

      return saved;
    },
    [cancel, client, draft, runDashboardAction],
  );

  const updateState = useCallback(
    (
      taskId: EntityId,
      input: { attentionState?: AttentionState; executionState?: ExecutionState },
    ) => runDashboardAction(`update:${taskId}`, () => client.updateTask(taskId, input)),
    [client, runDashboardAction],
  );

  const archive = useCallback(
    async (taskId: EntityId): Promise<boolean> => {
      const archived = await runDashboardAction(`archive:${taskId}`, () =>
        client.archiveTask(taskId),
      );

      if (archived) {
        onArchived?.(taskId);
      }

      return archived;
    },
    [client, onArchived, runDashboardAction],
  );

  const actions = useMemo<TaskEditControllerActions>(
    () => ({
      setDraft,
      patchDraft,
      start,
      cancel,
      save,
      updateState,
      archive,
    }),
    [archive, cancel, patchDraft, save, start, updateState],
  );

  return {
    state: {
      editingTaskId,
      draft,
    },
    actions,
  };
}
