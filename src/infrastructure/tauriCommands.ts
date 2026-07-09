import { invoke } from '@tauri-apps/api/core';
import type { EntityId } from '../domain/model';
import type {
  CreateTaskDashboardTaskInput,
  DiscoverTaskReposInput,
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
  RegisterTaskWorktreeInput,
  TaskDashboardClient,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/taskDashboardClient';
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type {
  TaskRunDetailClient,
  TaskRunDetailSnapshot,
} from '../application/taskRunDetailClient';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface AppMetadata {
  appName: string;
  storageMode: 'local-first';
  codexRuntime: 'adapter-pending' | 'tauri-codex-exec';
}

export interface BackendMaintenanceResult {
  status: 'current' | 'restarting' | 'failed';
  stale: boolean;
  checkedAt: string;
  newestSourcePath?: string;
  newestSourceModifiedAt?: string;
  executableModifiedAt?: string;
  message: string;
}

export interface BackendMaintenanceClient {
  checkAndReopenBackend(): Promise<BackendMaintenanceResult>;
}

export async function getAppMetadata(): Promise<AppMetadata> {
  return invoke<AppMetadata>('app_metadata');
}

export const tauriTaskDashboardClient: TaskDashboardClient = {
  async loadDashboard(): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('load_open_task_dashboard');
  },

  async registerWorktree(input: RegisterTaskWorktreeInput): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('register_task_worktree', { input });
  },

  async registerRepo(input: RegisterTaskRepoInput): Promise<TaskDashboardSnapshot> {
    return invoke<TaskDashboardSnapshot>('register_task_repo', { input });
  },

  async discoverRepos(input: DiscoverTaskReposInput): Promise<DiscoveredTaskRepo[]> {
    return invoke<DiscoveredTaskRepo[]>('discover_task_repos', { input });
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

export function createTauriRuntimeCommandClient(
  invokeCommand: TauriInvoke = invoke,
): RuntimeCommandClient {
  return {
    startCodexTaskRun(
      input: StartCodexTaskRunCommandInput,
    ): Promise<StartCodexTaskRunCommandResult> {
      return invokeCommand<StartCodexTaskRunCommandResult>('start_codex_task_run', { input });
    },
  };
}

export const tauriRuntimeCommandClient = createTauriRuntimeCommandClient();

export function createTauriBackendMaintenanceClient(
  invokeCommand: TauriInvoke = invoke,
): BackendMaintenanceClient {
  return {
    checkAndReopenBackend(): Promise<BackendMaintenanceResult> {
      return invokeCommand<BackendMaintenanceResult>('check_and_reopen_rust_backend');
    },
  };
}

export const tauriBackendMaintenanceClient = createTauriBackendMaintenanceClient();

export function createTauriTaskRunDetailClient(
  invokeCommand: TauriInvoke = invoke,
): TaskRunDetailClient {
  return {
    loadTaskRunDetail(taskId: EntityId): Promise<TaskRunDetailSnapshot> {
      return invokeCommand<TaskRunDetailSnapshot>('load_task_run_detail', { taskId });
    },
  };
}

export const tauriTaskRunDetailClient = createTauriTaskRunDetailClient();
