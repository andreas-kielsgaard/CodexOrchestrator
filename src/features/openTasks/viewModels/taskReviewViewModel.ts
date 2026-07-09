import type { DashboardGroup, DashboardGroupId, DashboardTask } from '../../../domain/dashboardProjection';
import type { AttentionState, EntityId, ExecutionState, Task } from '../../../domain/model';
import { compactPath } from '../../../app/viewModels/formatting';

export type OpenTaskGroupViewId = DashboardGroupId;
export type OpenTaskId = EntityId;
export type OpenTaskAttentionValue = AttentionState;
export type OpenTaskExecutionValue = ExecutionState;
export type OpenTaskPriorityValue = Task['priority'];
export type OpenTaskReviewBusyAction = `update:${string}` | `archive:${string}` | string | null;

export interface OpenTaskRunTaskViewModel {
  id: EntityId;
  title: string;
  summary: string;
  worktreePath?: string;
}

export interface OpenTaskRunFeedbackViewModel {
  status: 'running' | 'completed' | 'failed';
  message: string;
}

export interface OpenTaskEditDraftViewModel {
  projectId: EntityId;
  worktreeId: EntityId;
  title: string;
  summary: string;
  attentionState: OpenTaskAttentionValue;
  executionState: OpenTaskExecutionValue;
  priority: OpenTaskPriorityValue;
}

export interface OpenTaskCardViewModel {
  id: EntityId;
  title: string;
  summary: string;
  project: string;
  priority: OpenTaskPriorityValue;
  executionState: OpenTaskExecutionValue;
  attentionState: OpenTaskAttentionValue;
  repo?: string;
  branch?: string;
  worktreePath?: string;
  compactWorktreePath?: string;
  selected: boolean;
  editing: boolean;
  busy: boolean;
  runBusy: boolean;
  prompt: string;
  runAction?: OpenTaskRunFeedbackViewModel;
  runTask: OpenTaskRunTaskViewModel;
}

export interface OpenTaskGroupViewModel {
  id: OpenTaskGroupViewId;
  title: string;
  tasks: OpenTaskCardViewModel[];
}

export interface OpenTaskReviewViewModel {
  groups: OpenTaskGroupViewModel[];
}

export interface CreateOpenTaskReviewViewModelInput {
  groups: DashboardGroup[];
  busyAction: OpenTaskReviewBusyAction;
  selectedTaskId: EntityId | null;
  editingTaskId: EntityId | null;
  promptsByTaskId: Record<EntityId, string>;
  runActionsByTaskId: Record<EntityId, OpenTaskRunFeedbackViewModel>;
}

export function createOpenTaskReviewViewModel({
  groups,
  busyAction,
  selectedTaskId,
  editingTaskId,
  promptsByTaskId,
  runActionsByTaskId,
}: CreateOpenTaskReviewViewModelInput): OpenTaskReviewViewModel {
  return {
    groups: groups.map((group) => ({
      id: group.id,
      title: group.title,
      tasks: group.tasks.map((task) =>
        createOpenTaskCardViewModel({
          task,
          busyAction,
          selectedTaskId,
          editingTaskId,
          promptsByTaskId,
          runActionsByTaskId,
        }),
      ),
    })),
  };
}

interface CreateOpenTaskCardViewModelInput {
  task: DashboardTask;
  busyAction: OpenTaskReviewBusyAction;
  selectedTaskId: EntityId | null;
  editingTaskId: EntityId | null;
  promptsByTaskId: Record<EntityId, string>;
  runActionsByTaskId: Record<EntityId, OpenTaskRunFeedbackViewModel>;
}

function createOpenTaskCardViewModel({
  task,
  busyAction,
  selectedTaskId,
  editingTaskId,
  promptsByTaskId,
  runActionsByTaskId,
}: CreateOpenTaskCardViewModelInput): OpenTaskCardViewModel {
  const runAction = runActionsByTaskId[task.id];
  const runBusy = runAction?.status === 'running';
  const busy = runBusy || busyAction === `update:${task.id}` || busyAction === `archive:${task.id}`;

  return {
    id: task.id,
    title: task.title,
    summary: task.summary,
    project: task.project,
    priority: task.priority,
    executionState: task.executionState,
    attentionState: task.attentionState,
    ...(task.repo === undefined ? {} : { repo: task.repo }),
    ...(task.branch === undefined ? {} : { branch: task.branch }),
    ...(task.worktreePath === undefined
      ? {}
      : {
          worktreePath: task.worktreePath,
          compactWorktreePath: compactPath(task.worktreePath),
        }),
    selected: selectedTaskId === task.id,
    editing: editingTaskId === task.id,
    busy,
    runBusy,
    prompt: promptsByTaskId[task.id] ?? '',
    ...(runAction === undefined ? {} : { runAction }),
    runTask: {
      id: task.id,
      title: task.title,
      summary: task.summary,
      ...(task.worktreePath === undefined ? {} : { worktreePath: task.worktreePath }),
    },
  };
}
