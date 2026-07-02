import { invoke } from '@tauri-apps/api/core';
import type { EntityId } from '../domain/model';
import type {
  CreateTaskDashboardTaskInput,
  TaskDashboardClient,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/taskDashboardClient';

export interface AppMetadata {
  appName: string;
  storageMode: 'local-first';
  codexRuntime: 'adapter-pending';
}

export async function getAppMetadata(): Promise<AppMetadata> {
  return invoke<AppMetadata>('app_metadata');
}

export const tauriTaskDashboardClient: TaskDashboardClient = {
  async loadDashboard(): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('load_open_task_dashboard');
  },

  async createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('create_open_task', { input });
  },

  async updateTask(
    taskId: EntityId,
    input: UpdateTaskDashboardTaskInput,
  ): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('update_open_task', { taskId, input });
  },

  async archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('archive_open_task', { taskId });
  },
};
