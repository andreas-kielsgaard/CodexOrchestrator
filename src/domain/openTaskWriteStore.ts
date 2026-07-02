import type {
  AttentionState,
  DomainRecords,
  EntityId,
  ExecutionState,
  IsoDateTime,
  Task,
} from './model';
import type { OpenTaskDashboardStore } from './openTaskDashboardStore';

export interface IdProvider {
  nextId(): EntityId;
}

export interface TimeProvider {
  now(): IsoDateTime;
}

export interface CreateOpenTaskInput {
  projectId: EntityId;
  title: string;
  summary: string;
  repoId?: EntityId;
  branchId?: EntityId;
  worktreeId?: EntityId;
  conversationIds?: readonly EntityId[];
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
  dueAt?: IsoDateTime;
  snoozedUntil?: IsoDateTime;
}

export interface UpdateOpenTaskInput {
  projectId?: EntityId;
  repoId?: EntityId | null;
  branchId?: EntityId | null;
  worktreeId?: EntityId | null;
  conversationIds?: readonly EntityId[];
  title?: string;
  summary?: string;
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
  dueAt?: IsoDateTime | null;
  snoozedUntil?: IsoDateTime | null;
}

export interface OpenTaskWriteStore {
  createTask(input: CreateOpenTaskInput): Promise<Task>;
  updateTask(taskId: EntityId, input: UpdateOpenTaskInput): Promise<Task>;
  archiveTask(taskId: EntityId): Promise<Task>;
}

export class OpenTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Open task not found: ${taskId}`);
    this.name = 'OpenTaskNotFoundError';
  }
}

export class InMemoryOpenTaskWriteStore implements OpenTaskWriteStore, OpenTaskDashboardStore {
  private records: DomainRecords;

  constructor(
    records: DomainRecords,
    private readonly ids: IdProvider,
    private readonly clock: TimeProvider,
  ) {
    this.records = cloneDomainRecords(records);
  }

  async loadOpenTaskDashboardRecords(): Promise<DomainRecords> {
    return cloneDomainRecords(this.records);
  }

  snapshot(): DomainRecords {
    return cloneDomainRecords(this.records);
  }

  async createTask(input: CreateOpenTaskInput): Promise<Task> {
    const now = this.clock.now();
    const task: Task = {
      id: this.ids.nextId(),
      projectId: input.projectId,
      ...(input.repoId === undefined ? {} : { repoId: input.repoId }),
      ...(input.branchId === undefined ? {} : { branchId: input.branchId }),
      ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
      conversationIds: [...(input.conversationIds ?? [])],
      title: input.title,
      summary: input.summary,
      executionState: input.executionState ?? 'draft',
      attentionState: input.attentionState ?? 'consider_later',
      priority: input.priority ?? 'normal',
      ...(input.dueAt === undefined ? {} : { dueAt: input.dueAt }),
      ...(input.snoozedUntil === undefined ? {} : { snoozedUntil: input.snoozedUntil }),
      createdAt: now,
      updatedAt: now,
    };

    this.records = {
      ...this.records,
      tasks: [...this.records.tasks, task],
    };

    return cloneTask(task);
  }

  async updateTask(taskId: EntityId, input: UpdateOpenTaskInput): Promise<Task> {
    const taskIndex = this.findTaskIndex(taskId);
    const updatedTask = applyTaskUpdate(this.records.tasks[taskIndex], input, this.clock.now());

    this.records = {
      ...this.records,
      tasks: this.records.tasks.map((task, index) => (index === taskIndex ? updatedTask : task)),
    };

    return cloneTask(updatedTask);
  }

  async archiveTask(taskId: EntityId): Promise<Task> {
    return this.updateTask(taskId, { executionState: 'archived' });
  }

  private findTaskIndex(taskId: EntityId): number {
    const taskIndex = this.records.tasks.findIndex((task) => task.id === taskId);

    if (taskIndex === -1) {
      throw new OpenTaskNotFoundError(taskId);
    }

    return taskIndex;
  }
}

export function applyTaskUpdate(
  task: Task,
  input: UpdateOpenTaskInput,
  updatedAt: IsoDateTime,
): Task {
  const updatedTask: Task = {
    ...task,
    ...(input.projectId === undefined ? {} : { projectId: input.projectId }),
    ...(input.conversationIds === undefined ? {} : { conversationIds: [...input.conversationIds] }),
    ...(input.title === undefined ? {} : { title: input.title }),
    ...(input.summary === undefined ? {} : { summary: input.summary }),
    ...(input.executionState === undefined ? {} : { executionState: input.executionState }),
    ...(input.attentionState === undefined ? {} : { attentionState: input.attentionState }),
    ...(input.priority === undefined ? {} : { priority: input.priority }),
    updatedAt,
  };

  applyOptionalField(updatedTask, 'repoId', input.repoId);
  applyOptionalField(updatedTask, 'branchId', input.branchId);
  applyOptionalField(updatedTask, 'worktreeId', input.worktreeId);
  applyOptionalField(updatedTask, 'dueAt', input.dueAt);
  applyOptionalField(updatedTask, 'snoozedUntil', input.snoozedUntil);

  return updatedTask;
}

function applyOptionalField<T extends keyof Task>(
  task: Task,
  field: T,
  value: Task[T] | null | undefined,
): void {
  if (value === undefined) {
    return;
  }

  if (value === null) {
    delete task[field];
    return;
  }

  task[field] = value;
}

function cloneDomainRecords(records: DomainRecords): DomainRecords {
  return {
    projects: records.projects.map((project) => ({ ...project })),
    repos: records.repos.map((repo) => ({ ...repo })),
    branches: records.branches.map((branch) => ({ ...branch })),
    worktrees: records.worktrees.map((worktree) => ({ ...worktree })),
    conversations: records.conversations.map((conversation) => ({ ...conversation })),
    tasks: records.tasks.map(cloneTask),
    taskRuns: records.taskRuns.map((taskRun) => ({ ...taskRun })),
    artifacts: records.artifacts.map((artifact) => ({ ...artifact })),
    validationRuns: records.validationRuns.map((validationRun) => ({ ...validationRun })),
    events: records.events.map((event) => ({ ...event, payload: { ...event.payload } })),
  };
}

function cloneTask(task: Task): Task {
  return {
    ...task,
    conversationIds: [...task.conversationIds],
  };
}
