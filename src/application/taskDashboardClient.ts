import {
  dashboardGroupOrder,
  projectOpenTaskDashboard,
  type DashboardGroup,
} from '../domain/dashboardProjection';
import type { AttentionState, EntityId, ExecutionState, Task } from '../domain/model';
import type { OpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import type { CreateOpenTaskInput, OpenTaskWriteStore } from '../domain/openTaskWriteStore';

export interface TaskDashboardProject {
  id: EntityId;
  name: string;
}

export interface TaskDashboardSnapshot {
  groups: DashboardGroup[];
  projects: TaskDashboardProject[];
  totalOpenTasks: number;
}

export interface CreateTaskDashboardTaskInput {
  projectId: EntityId;
  title: string;
  summary: string;
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
}

export interface UpdateTaskDashboardTaskInput {
  title?: string;
  summary?: string;
  executionState?: ExecutionState;
  attentionState?: AttentionState;
  priority?: Task['priority'];
}

export interface TaskDashboardClient {
  loadDashboard(): Promise<TaskDashboardSnapshot>;
  createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
  updateTask(taskId: EntityId, input: UpdateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
  archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot>;
}

export interface StoreBackedTaskDashboardClientStores {
  dashboard: OpenTaskDashboardStore;
  write: OpenTaskWriteStore;
}

export function createStoreBackedTaskDashboardClient(
  stores: StoreBackedTaskDashboardClientStores,
): TaskDashboardClient {
  return {
    async loadDashboard(): Promise<TaskDashboardSnapshot> {
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot> {
      await stores.write.createTask(normalizeCreateTaskInput(input));
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async updateTask(
      taskId: EntityId,
      input: UpdateTaskDashboardTaskInput,
    ): Promise<TaskDashboardSnapshot> {
      await stores.write.updateTask(taskId, input);
      return loadTaskDashboardSnapshot(stores.dashboard);
    },

    async archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot> {
      await stores.write.archiveTask(taskId);
      return loadTaskDashboardSnapshot(stores.dashboard);
    },
  };
}

export function emptyTaskDashboardSnapshot(): TaskDashboardSnapshot {
  return {
    groups: dashboardGroupOrder.map((group) => ({ ...group, tasks: [] })),
    projects: [],
    totalOpenTasks: 0,
  };
}

async function loadTaskDashboardSnapshot(
  store: OpenTaskDashboardStore,
): Promise<TaskDashboardSnapshot> {
  const records = await store.loadOpenTaskDashboardRecords();
  const groups = projectOpenTaskDashboard(records);

  return {
    groups,
    projects: records.projects
      .map((project) => ({ id: project.id, name: project.name }))
      .sort((left, right) => left.name.localeCompare(right.name)),
    totalOpenTasks: groups.reduce((total, group) => total + group.tasks.length, 0),
  };
}

function normalizeCreateTaskInput(input: CreateTaskDashboardTaskInput): CreateOpenTaskInput {
  return {
    projectId: input.projectId,
    title: input.title,
    summary: input.summary,
    executionState: input.executionState ?? 'draft',
    attentionState: input.attentionState ?? 'needs_action_now',
    priority: input.priority ?? 'normal',
  };
}
