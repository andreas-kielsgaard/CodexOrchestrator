import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
  StartAgentSessionCommandInput,
  StartAgentSessionCommandOptions,
  StartAgentSessionOutputChunk,
  StartAgentSessionCommandResult,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type {
  TaskRunDetailClient,
  TaskRunDetailSnapshot,
} from '../application/taskRunDetailClient';
import {
  fallbackRuntimeInfo,
  parseCodexDoctorReport,
  parseCodexModelCatalog,
  type CodexRuntimeInfo,
} from '../application/codexRuntimeInfoProvider';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type TauriUnlisten = () => void;
type TauriListen = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<TauriUnlisten>;

interface TauriAgentSessionOutputEvent {
  streamId: string;
  stream: StartAgentSessionOutputChunk['stream'];
  content: string;
}

interface TauriAgentSessionCompletedEvent {
  streamId: string;
  result: StartAgentSessionCommandResult;
}

interface StartAgentSessionStartedCommandResult {
  sessionId: EntityId;
  streamId: EntityId;
  status: 'running';
  command: string;
  args: string[];
  startedAt: string;
}

interface CodexRuntimeInfoCommandResult {
  doctorStdout: string;
  modelCatalogStdout: string;
}

export interface AppMetadata {
  appName: string;
  storageMode: 'local-first';
  codexRuntime: 'adapter-pending' | 'tauri-codex-exec';
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
  listenToEvent: TauriListen = listen,
): RuntimeCommandClient {
  return {
    startCodexTaskRun(
      input: StartCodexTaskRunCommandInput,
    ): Promise<StartCodexTaskRunCommandResult> {
      return invokeCommand<StartCodexTaskRunCommandResult>('start_codex_task_run', { input });
    },
    async startAgentSession(
      input: StartAgentSessionCommandInput,
      options?: StartAgentSessionCommandOptions,
    ): Promise<StartAgentSessionCommandResult> {
      const streamId = input.streamId ?? crypto.randomUUID();
      const shouldStreamOutput = options?.onOutput !== undefined;
      let completionUnlisten: TauriUnlisten | undefined;
      let outputUnlisten: TauriUnlisten | undefined;

      try {
        let resolveCompletion: (result: StartAgentSessionCommandResult) => void = () => {};
        const completion = new Promise<StartAgentSessionCommandResult>((resolve) => {
          resolveCompletion = resolve;
        });
        completionUnlisten = await listenToEvent<TauriAgentSessionCompletedEvent>(
          'agent-session-cli-completed',
          (event) => {
            if (event.payload.streamId === streamId) {
              resolveCompletion(event.payload.result);
            }
          },
        );

        if (shouldStreamOutput) {
          outputUnlisten = await listenToEvent<TauriAgentSessionOutputEvent>(
            'agent-session-cli-output',
            (event) => {
              if (event.payload.streamId === streamId) {
                options.onOutput?.({
                  stream: event.payload.stream,
                  content: event.payload.content,
                });
              }
            },
          );
        }

        await invokeCommand<StartAgentSessionStartedCommandResult>('start_agent_session', {
          input: { ...input, streamId },
        });

        return await completion;
      } finally {
        outputUnlisten?.();
        completionUnlisten?.();
      }
    },
    loadAgentSession(sessionId: EntityId): Promise<StartAgentSessionCommandResult | null> {
      return invokeCommand<StartAgentSessionCommandResult | null>('load_agent_session', {
        sessionId,
      });
    },
  };
}

export const tauriRuntimeCommandClient = createTauriRuntimeCommandClient();

export async function loadTauriCodexRuntimeInfo(): Promise<CodexRuntimeInfo> {
  const result = await invoke<CodexRuntimeInfoCommandResult>('load_codex_runtime_info');
  const doctorInfo = parseCodexDoctorReport(result.doctorStdout);
  const catalogInfo = parseCodexModelCatalog(result.modelCatalogStdout);

  if (!doctorInfo && !catalogInfo) {
    return fallbackRuntimeInfo;
  }

  if (!catalogInfo) {
    return {
      ...fallbackRuntimeInfo,
      ...doctorInfo,
      source: doctorInfo ? 'codex-doctor-and-debug-models' : 'fallback',
    };
  }

  return {
    ...catalogInfo,
    ...doctorInfo,
    recommendedModel:
      doctorInfo?.configuredModel ??
      catalogInfo.recommendedModel ??
      fallbackRuntimeInfo.recommendedModel,
    source: doctorInfo ? 'codex-doctor-and-debug-models' : 'codex-debug-models',
  };
}

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
