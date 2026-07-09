import type {
  CreateTaskDashboardTaskInput,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/commands/taskDashboardClient';
import { emptyTaskDashboardSnapshot } from '../application/commands/taskDashboardClient';
import type { EntityId } from '../domain/model';

export type {
  CreateTaskDashboardTaskInput,
  TaskDashboardProject,
  TaskDashboardRepo,
  TaskDashboardSnapshot,
  TaskDashboardWorktreeAnchor,
  UpdateTaskDashboardTaskInput,
} from '../application/commands/taskDashboardClient';

export interface LoadOpenTaskDashboardCapability {
  loadDashboard(): Promise<TaskDashboardSnapshot>;
}

export interface CreateOpenTaskCapability {
  createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
}

export interface UpdateOpenTaskCapability {
  updateTask(taskId: EntityId, input: UpdateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot>;
}

export interface ArchiveOpenTaskCapability {
  archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot>;
}

export type OpenTaskDashboardCapability = LoadOpenTaskDashboardCapability &
  CreateOpenTaskCapability &
  UpdateOpenTaskCapability &
  ArchiveOpenTaskCapability;

export function emptyOpenTaskDashboardSnapshot(): TaskDashboardSnapshot {
  return emptyTaskDashboardSnapshot();
}
