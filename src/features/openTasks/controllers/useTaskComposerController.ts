import { useCallback, useLayoutEffect, useMemo, useState, type FormEvent } from 'react';
import type {
  CreateOpenTaskCapability,
  CreateTaskDashboardTaskInput,
  TaskDashboardSnapshot,
} from '../../../capabilities/openTaskDashboard';
import type { AttentionState, EntityId, ExecutionState, Task } from '../../../domain/model';
import { errorMessage } from '../../../app/viewModels/formatting';
import {
  nextCreateFormAnchorDefaults,
  selectedWorktreeAnchor,
  type TaskAnchorDraft,
} from '../viewModels/taskFormViewModel';

export interface TaskComposerDraft extends TaskAnchorDraft {
  title: string;
  summary: string;
  attentionState: AttentionState;
  executionState: ExecutionState;
  priority: Task['priority'];
}

export type TaskComposerBusyAction = 'create' | null;

export interface TaskComposerControllerState {
  draft: TaskComposerDraft;
  busyAction: TaskComposerBusyAction;
  error: string | null;
  canCreate: boolean;
  createDisabled: boolean;
}

export interface TaskComposerControllerActions {
  setDraft(draft: TaskComposerDraft): void;
  patchDraft(patch: Partial<TaskComposerDraft>): void;
  selectProject(projectId: EntityId): void;
  selectWorktree(worktreeId: EntityId): void;
  submit(event?: FormEvent<HTMLFormElement>): void;
  createTask(): Promise<boolean>;
  resetDraft(anchor?: Partial<TaskAnchorDraft>): void;
  clearError(): void;
}

export interface TaskComposerController {
  state: TaskComposerControllerState;
  actions: TaskComposerControllerActions;
}

export interface UseTaskComposerControllerInput {
  client: CreateOpenTaskCapability;
  snapshot: TaskDashboardSnapshot;
  onSnapshot(snapshot: TaskDashboardSnapshot): void;
  initialDraft?: Partial<TaskComposerDraft>;
}

export const initialTaskComposerDraft: TaskComposerDraft = {
  projectId: '',
  worktreeId: '',
  title: '',
  summary: '',
  attentionState: 'needs_action_now',
  executionState: 'draft',
  priority: 'normal',
};

export function useTaskComposerController({
  client,
  snapshot,
  onSnapshot,
  initialDraft,
}: UseTaskComposerControllerInput): TaskComposerController {
  const [draft, setDraft] = useState<TaskComposerDraft>(() => ({
    ...initialTaskComposerDraft,
    ...initialDraft,
  }));
  const [busyAction, setBusyAction] = useState<TaskComposerBusyAction>(null);
  const [error, setError] = useState<string | null>(null);

  const canCreate = snapshot.projects.length > 0 && busyAction === null;
  const createDisabled =
    !canCreate || draft.title.trim().length === 0 || draft.summary.trim().length === 0;

  useLayoutEffect(() => {
    setDraft((current) => ({
      ...current,
      ...nextCreateFormAnchorDefaults(current, snapshot),
    }));
  }, [snapshot]);

  const patchDraft = useCallback((patch: Partial<TaskComposerDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
  }, []);

  const selectProject = useCallback(
    (projectId: EntityId) => {
      patchDraft({ projectId });
    },
    [patchDraft],
  );

  const selectWorktree = useCallback(
    (worktreeId: EntityId) => {
      const anchor = selectedWorktreeAnchor(snapshot, worktreeId);

      setDraft((current) => ({
        ...current,
        worktreeId,
        projectId: anchor?.projectId ?? current.projectId,
      }));
    },
    [snapshot],
  );

  const resetDraft = useCallback((anchor?: Partial<TaskAnchorDraft>) => {
    setDraft((current) => ({
      ...initialTaskComposerDraft,
      projectId: anchor?.projectId ?? current.projectId,
      worktreeId: anchor?.worktreeId ?? current.worktreeId,
    }));
  }, []);

  const createTask = useCallback(async (): Promise<boolean> => {
    const title = draft.title.trim();
    const summary = draft.summary.trim();

    if (!title || !summary || !draft.projectId || busyAction !== null) {
      return false;
    }

    setBusyAction('create');
    setError(null);

    try {
      const selectedAnchor = selectedWorktreeAnchor(snapshot, draft.worktreeId);
      const input: CreateTaskDashboardTaskInput = {
        projectId: selectedAnchor?.projectId ?? draft.projectId,
        repoId: selectedAnchor?.repoId,
        branchId: selectedAnchor?.branchId,
        worktreeId: selectedAnchor?.id,
        title,
        summary,
        attentionState: draft.attentionState,
        executionState: draft.executionState,
        priority: draft.priority,
      };
      const nextSnapshot = await client.createTask(input);

      onSnapshot(nextSnapshot);
      resetDraft(nextCreateFormAnchorDefaults(draft, nextSnapshot));
      return true;
    } catch (caught) {
      setError(errorMessage(caught));
      return false;
    } finally {
      setBusyAction(null);
    }
  }, [busyAction, client, draft, onSnapshot, resetDraft, snapshot]);

  const submit = useCallback(
    (event?: FormEvent<HTMLFormElement>) => {
      event?.preventDefault();
      void createTask();
    },
    [createTask],
  );

  const actions = useMemo<TaskComposerControllerActions>(
    () => ({
      setDraft,
      patchDraft,
      selectProject,
      selectWorktree,
      submit,
      createTask,
      resetDraft,
      clearError: () => setError(null),
    }),
    [createTask, patchDraft, resetDraft, selectProject, selectWorktree, submit],
  );

  return {
    state: {
      draft,
      busyAction,
      error,
      canCreate,
      createDisabled,
    },
    actions,
  };
}
