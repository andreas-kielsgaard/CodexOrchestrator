import type { Artifact } from '../../domain/model';
import type { ArtifactStore } from '../../domain/artifactStore';
import type { ConversationStore, CreateConversationInput } from '../../domain/conversationStore';
import type { EventStore } from '../../domain/eventStore';
import type { Conversation, EntityId, Event, IsoDateTime, Task, TaskRun } from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import type { OpenTaskWriteStore } from '../../domain/openTaskWriteStore';
import type { TaskRunStore } from '../../domain/taskRunStore';

export interface TaskRunLifecycleRecorder {
  readonly openTaskDashboardStore: OpenTaskDashboardStore;
  readonly openTaskWriteStore: OpenTaskWriteStore;
  readonly taskRunStore: TaskRunStore;
  readonly conversationStore: ConversationStore;
  readonly artifactStore: ArtifactStore;
  readonly eventStore: EventStore;
}

export interface StartTaskRunLifecycleInput {
  taskId: EntityId;
  worktreeId?: EntityId;
  startedAt?: IsoDateTime;
  conversation?: StartTaskRunConversationInput;
}

export interface StartTaskRunConversationInput {
  title: string;
  externalThreadId?: string;
  summary?: string;
}

export interface StartedTaskRunLifecycle {
  task: Task;
  taskRun: TaskRun;
  conversation?: Conversation;
  event: Event;
}

export interface CompleteTaskRunLifecycleInput {
  taskId: EntityId;
  taskRunId: EntityId;
  completedAt?: IsoDateTime;
  exitCode?: number;
  finalResponse?: FinalResponseArtifactInput;
}

export interface FinalResponseArtifactInput {
  title?: string;
  content: string;
}

export interface CompletedTaskRunLifecycle {
  task: Task;
  taskRun: TaskRun;
  artifact?: Artifact;
  event: Event;
}

export interface FailTaskRunLifecycleInput {
  taskId: EntityId;
  taskRunId: EntityId;
  completedAt?: IsoDateTime;
  exitCode?: number;
  error?: string;
}

export interface FailedTaskRunLifecycle {
  task: Task;
  taskRun: TaskRun;
  event: Event;
}

export class TaskRunLifecycleTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Task run lifecycle task not found: ${taskId}`);
    this.name = 'TaskRunLifecycleTaskNotFoundError';
  }
}

export class TaskRunLifecycleTaskRunNotFoundForTaskError extends Error {
  constructor(taskId: EntityId, taskRunId: EntityId) {
    super(`Task run lifecycle task run not found for task: ${taskRunId} for ${taskId}`);
    this.name = 'TaskRunLifecycleTaskRunNotFoundForTaskError';
  }
}

export async function startTaskRunLifecycle(
  recorder: TaskRunLifecycleRecorder,
  input: StartTaskRunLifecycleInput,
): Promise<StartedTaskRunLifecycle> {
  const existingTask = await requireTask(recorder.openTaskDashboardStore, input.taskId);
  let taskRun = await recorder.taskRunStore.createTaskRun({
    taskId: existingTask.id,
    executionState: 'running',
    ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
  });

  const conversationInput = input.conversation;
  const conversation =
    conversationInput === undefined
      ? undefined
      : await recorder.conversationStore.createConversation(
          createLifecycleConversationInput(existingTask.id, taskRun.id, conversationInput),
        );

  if (conversation !== undefined) {
    taskRun = await recorder.taskRunStore.updateTaskRun(taskRun.id, {
      conversationId: conversation.id,
    });
  }

  const task = await recorder.openTaskWriteStore.updateTask(existingTask.id, {
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    conversationIds:
      conversation === undefined
        ? existingTask.conversationIds
        : [...existingTask.conversationIds, conversation.id],
  });

  const event = await recorder.eventStore.appendEvent({
    kind: 'run_started',
    projectId: existingTask.projectId,
    taskId: existingTask.id,
    taskRunId: taskRun.id,
    ...(conversation === undefined ? {} : { conversationId: conversation.id }),
    payload: {
      taskId: existingTask.id,
      taskRunId: taskRun.id,
      ...(taskRun.worktreeId === undefined ? {} : { worktreeId: taskRun.worktreeId }),
      ...(taskRun.startedAt === undefined ? {} : { startedAt: taskRun.startedAt }),
      ...(conversation === undefined ? {} : { conversationId: conversation.id }),
    },
  });

  return { task, taskRun, ...(conversation === undefined ? {} : { conversation }), event };
}

export async function completeTaskRunLifecycle(
  recorder: TaskRunLifecycleRecorder,
  input: CompleteTaskRunLifecycleInput,
): Promise<CompletedTaskRunLifecycle> {
  const existingTask = await requireTask(recorder.openTaskDashboardStore, input.taskId);
  await requireTaskRunForTask(recorder.taskRunStore, existingTask.id, input.taskRunId);
  const taskRun = await recorder.taskRunStore.updateTaskRun(input.taskRunId, {
    executionState: 'completed',
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    ...(input.exitCode === undefined ? {} : { exitCode: input.exitCode }),
  });
  const artifact =
    input.finalResponse === undefined
      ? undefined
      : await recorder.artifactStore.createArtifact({
          kind: 'final_response',
          title: input.finalResponse.title ?? 'Final response',
          taskId: existingTask.id,
          taskRunId: taskRun.id,
          ...(taskRun.conversationId === undefined
            ? {}
            : { conversationId: taskRun.conversationId }),
          content: input.finalResponse.content,
        });
  const task = await recorder.openTaskWriteStore.updateTask(existingTask.id, {
    executionState: 'completed',
    attentionState: 'needs_review',
  });
  const event = await recorder.eventStore.appendEvent({
    kind: 'run_completed',
    projectId: existingTask.projectId,
    taskId: existingTask.id,
    taskRunId: taskRun.id,
    ...(taskRun.conversationId === undefined ? {} : { conversationId: taskRun.conversationId }),
    ...(artifact === undefined ? {} : { artifactId: artifact.id }),
    payload: {
      outcome: 'completed',
      taskId: existingTask.id,
      taskRunId: taskRun.id,
      ...(taskRun.completedAt === undefined ? {} : { completedAt: taskRun.completedAt }),
      ...(taskRun.exitCode === undefined ? {} : { exitCode: taskRun.exitCode }),
      ...(artifact === undefined ? {} : { artifactId: artifact.id }),
    },
  });

  return { task, taskRun, ...(artifact === undefined ? {} : { artifact }), event };
}

export async function failTaskRunLifecycle(
  recorder: TaskRunLifecycleRecorder,
  input: FailTaskRunLifecycleInput,
): Promise<FailedTaskRunLifecycle> {
  const existingTask = await requireTask(recorder.openTaskDashboardStore, input.taskId);
  await requireTaskRunForTask(recorder.taskRunStore, existingTask.id, input.taskRunId);
  const taskRun = await recorder.taskRunStore.updateTaskRun(input.taskRunId, {
    executionState: 'failed',
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    ...(input.exitCode === undefined ? {} : { exitCode: input.exitCode }),
  });
  const task = await recorder.openTaskWriteStore.updateTask(existingTask.id, {
    executionState: 'failed',
    attentionState: 'needs_action_now',
  });
  const event = await recorder.eventStore.appendEvent({
    kind: 'run_completed',
    projectId: existingTask.projectId,
    taskId: existingTask.id,
    taskRunId: taskRun.id,
    ...(taskRun.conversationId === undefined ? {} : { conversationId: taskRun.conversationId }),
    payload: {
      outcome: 'failed',
      taskId: existingTask.id,
      taskRunId: taskRun.id,
      ...(taskRun.completedAt === undefined ? {} : { completedAt: taskRun.completedAt }),
      ...(taskRun.exitCode === undefined ? {} : { exitCode: taskRun.exitCode }),
      ...(input.error === undefined ? {} : { error: input.error }),
    },
  });

  return { task, taskRun, event };
}

async function requireTask(store: OpenTaskDashboardStore, taskId: EntityId): Promise<Task> {
  const records = await store.loadOpenTaskDashboardRecords();
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new TaskRunLifecycleTaskNotFoundError(taskId);
  }

  return {
    ...task,
    conversationIds: [...task.conversationIds],
  };
}

async function requireTaskRunForTask(
  store: TaskRunStore,
  taskId: EntityId,
  taskRunId: EntityId,
): Promise<void> {
  const taskRuns = await store.queryTaskRuns({ taskId });
  const taskRun = taskRuns.find((candidate) => candidate.id === taskRunId);

  if (taskRun === undefined) {
    throw new TaskRunLifecycleTaskRunNotFoundForTaskError(taskId, taskRunId);
  }
}

function createLifecycleConversationInput(
  taskId: EntityId,
  taskRunId: EntityId,
  input: StartTaskRunConversationInput,
): CreateConversationInput {
  return {
    provider: 'codex',
    taskId,
    taskRunId,
    title: input.title,
    ...(input.externalThreadId === undefined ? {} : { externalThreadId: input.externalThreadId }),
    ...(input.summary === undefined ? {} : { summary: input.summary }),
  };
}