import type { BackendMaintenanceCapability } from '../../../capabilities/backendMaintenance';
import type { OpenTaskDashboardCapability } from '../../../capabilities/openTaskDashboard';
import type { RepoOnboardingCapability } from '../../../capabilities/repoOnboarding';
import type { RuntimeHealthCapability } from '../../../capabilities/runtimeHealth';
import type { TaskRunDetailCapability } from '../../../capabilities/taskRunDetail';
import type { TaskRunLaunchCapability } from '../../../capabilities/taskRunLaunch';
import { useBackendMaintenanceController } from '../../../app/controllers/useBackendMaintenanceController';
import { useRuntimeHealthController } from '../../../app/controllers/useRuntimeHealthController';
import { createBackendMaintenanceViewModel } from '../../../app/viewModels/backendMaintenanceViewModel';
import { createDiscoveredRepoOptions } from '../viewModels/repoSetupViewModel';
import {
  createTaskComposerProjectOptions,
  createTaskComposerWorktreeOptions,
} from '../viewModels/taskFormViewModel';
import { createTaskRunDetailPanelViewModel } from '../viewModels/taskDetailViewModel';
import { createOpenTaskReviewViewModel } from '../viewModels/taskReviewViewModel';
import type {
  OpenTasksScreenActions,
  OpenTasksScreenViewModel,
} from '../viewModels/openTasksScreenViewModel';
import { useOpenTaskDashboardController } from './useOpenTaskDashboardController';
import { useRepoOnboardingController } from './useRepoOnboardingController';
import { useTaskComposerController } from './useTaskComposerController';
import { useTaskDetailController } from './useTaskDetailController';
import { useTaskEditController, type TaskEditDraft } from './useTaskEditController';
import { useTaskRunController } from './useTaskRunController';

export interface OpenTasksFeatureControllerOptions {
  taskDashboardClient: OpenTaskDashboardCapability & RepoOnboardingCapability;
  taskRunDetailClient: TaskRunDetailCapability;
  runtimeCommandClient: TaskRunLaunchCapability;
  runtimeStatusClient?: RuntimeHealthCapability;
  backendMaintenanceClient?: BackendMaintenanceCapability;
  startupLoadTimeoutMs?: number;
  reloadApp?(): void;
}

export interface OpenTasksFeatureController {
  view: OpenTasksScreenViewModel;
  actions: OpenTasksScreenActions;
}

const initialCreateForm: TaskEditDraft = {
  projectId: '',
  worktreeId: '',
  title: '',
  summary: '',
  attentionState: 'needs_action_now',
  executionState: 'draft',
  priority: 'normal',
};

export function useOpenTasksFeatureController({
  taskDashboardClient,
  taskRunDetailClient,
  runtimeCommandClient,
  runtimeStatusClient,
  backendMaintenanceClient,
  startupLoadTimeoutMs,
  reloadApp = () => window.location.reload(),
}: OpenTasksFeatureControllerOptions): OpenTasksFeatureController {
  const {
    snapshot,
    busyAction: dashboardBusyAction,
    error: dashboardError,
    hasLoadedDashboard,
    applySnapshot,
    loadDashboard,
    runClientAction,
  } = useOpenTaskDashboardController({ taskDashboardClient, startupLoadTimeoutMs });
  const repoOnboarding = useRepoOnboardingController({
    client: taskDashboardClient,
    onSnapshot: applySnapshot,
  });
  const taskComposer = useTaskComposerController({
    client: taskDashboardClient,
    snapshot,
    onSnapshot: applySnapshot,
    initialDraft: initialCreateForm,
  });
  const runtimeHealth = useRuntimeHealthController({ client: runtimeStatusClient });
  const backendMaintenance = useBackendMaintenanceController({
    client: backendMaintenanceClient,
  });
  const taskDetail = useTaskDetailController({ client: taskRunDetailClient });
  const detail = taskDetail.state;
  const taskEdit = useTaskEditController({
    client: taskDashboardClient,
    runDashboardAction: runClientAction,
    initialDraft: initialCreateForm,
    onArchived: (taskId) => {
      if (detail.taskId === taskId) {
        taskDetail.actions.close();
      }
    },
  });
  const taskRun = useTaskRunController({
    runtimeCommandClient,
    dashboardClient: taskDashboardClient,
    selectedDetailTaskId: detail.taskId,
    onSnapshot: applySnapshot,
    onLoadTaskDetail: taskDetail.actions.load,
  });

  const busyAction =
    dashboardBusyAction ?? repoOnboarding.state.busyAction ?? taskComposer.state.busyAction;
  const error =
    dashboardError ?? repoOnboarding.state.error ?? taskComposer.state.error ?? taskRun.state.error;
  const staleNoticeMessage = runtimeHealth.state.staleNoticeVisible
    ? runtimeHealth.state.staleNoticeMessage
    : null;

  return {
    view: {
      hasLoadedDashboard,
      startup: {
        loading: busyAction === 'load',
        error,
      },
      sidebar: {
        backendMaintenance: createBackendMaintenanceViewModel({
          status: backendMaintenance.state.status,
          message: backendMaintenance.state.message,
          ...(backendMaintenance.state.result?.newestSourcePath === undefined
            ? {}
            : { newestSourcePath: backendMaintenance.state.result.newestSourcePath }),
          available: backendMaintenance.state.available,
        }),
      },
      header: {
        totalOpenTasks: snapshot.totalOpenTasks,
        projectCount: snapshot.projects.length,
        worktreeCount: snapshot.worktreeAnchors.length,
        busy: busyAction !== null,
      },
      staleNoticeMessage,
      error,
      repoSetup: {
        form: repoOnboarding.state.draft,
        discoveredRepos: createDiscoveredRepoOptions(repoOnboarding.state.discoveredRepos),
        addBusy: busyAction === 'register-repo',
        scanBusy: busyAction === 'discover-repos',
        available: repoOnboarding.state.registerAvailable,
        scanAvailable: repoOnboarding.state.discoverAvailable,
      },
      composer: {
        form: taskComposer.state.draft,
        projects: createTaskComposerProjectOptions(snapshot.projects),
        worktrees: createTaskComposerWorktreeOptions(snapshot.worktreeAnchors),
        busy: busyAction !== null,
        canCreate: snapshot.projects.length > 0 && busyAction === null,
      },
      review: createOpenTaskReviewViewModel({
        groups: snapshot.groups,
        busyAction,
        selectedTaskId: taskDetail.state.taskId,
        editingTaskId: taskEdit.state.editingTaskId,
        promptsByTaskId: taskRun.state.prompts,
        runActionsByTaskId: taskRun.state.actionsByTaskId,
      }),
      editDraft: taskEdit.state.draft,
      taskDetail: createTaskRunDetailPanelViewModel(taskDetail.state),
    },
    actions: {
      retryStartup: () => void loadDashboard(),
      checkBackend: backendMaintenance.actions.checkAndReopenBackend,
      reloadDashboard: () => void loadDashboard(),
      refreshAppRuntime: () => {
        void (async () => {
          await runtimeHealth.actions.clearStale();
          reloadApp();
        })();
      },
      dismissStaleNotice: runtimeHealth.actions.dismissStaleNotice,
      changeRepoSetup: repoOnboarding.actions.setDraft,
      submitRepoSetup: repoOnboarding.actions.submitRegister,
      scanRepos: repoOnboarding.actions.submitDiscover,
      addDiscoveredRepo: (repoRootPath) =>
        void repoOnboarding.actions.registerRepo({ repoRootPath }),
      submitComposer: taskComposer.actions.submit,
      selectComposerProject: taskComposer.actions.selectProject,
      selectComposerWorktree: taskComposer.actions.selectWorktree,
      changeComposer: taskComposer.actions.patchDraft,
      changeEditDraft: taskEdit.actions.setDraft,
      saveEdit: (taskId) => void taskEdit.actions.save(taskId),
      cancelEdit: taskEdit.actions.cancel,
      changePrompt: taskRun.actions.updatePrompt,
      startRun: taskRun.actions.startRun,
      updateAttention: (taskId, attentionState) =>
        void taskEdit.actions.updateState(taskId, { attentionState }),
      updateExecution: (taskId, executionState) =>
        void taskEdit.actions.updateState(taskId, { executionState }),
      openDetail: (taskId) => void taskDetail.actions.load(taskId),
      editTask: taskEdit.actions.start,
      archiveTask: (taskId) => void taskEdit.actions.archive(taskId),
      closeTaskDetail: taskDetail.actions.close,
      reloadTaskDetail: () => void taskDetail.actions.reload(),
    },
  };
}
