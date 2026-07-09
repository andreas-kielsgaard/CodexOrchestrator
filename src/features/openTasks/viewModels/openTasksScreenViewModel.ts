import type { FormEvent } from 'react';
import type { BackendMaintenanceViewModel } from '../../../app/viewModels/backendMaintenanceViewModel';
import type { DiscoveredRepoOption, RepoSetupFormViewModel } from './repoSetupViewModel';
import type {
  TaskComposerFormViewModel,
  TaskComposerProjectOption,
  TaskComposerWorktreeOption,
} from './taskFormViewModel';
import type {
  OpenTaskAttentionValue,
  OpenTaskCardViewModel,
  OpenTaskEditDraftViewModel,
  OpenTaskExecutionValue,
  OpenTaskId,
  OpenTaskReviewViewModel,
  OpenTaskRunTaskViewModel,
} from './taskReviewViewModel';
import type { TaskRunDetailPanelViewModel } from './taskDetailViewModel';

export interface OpenTasksScreenViewModel {
  hasLoadedDashboard: boolean;
  startup: OpenTasksStartupViewModel;
  sidebar: OpenTasksSidebarViewModel;
  header: OpenTasksHeaderViewModel;
  staleNoticeMessage: string | null;
  error: string | null;
  repoSetup: OpenTasksRepoSetupViewModel;
  composer: OpenTasksComposerViewModel;
  review: OpenTaskReviewViewModel;
  editDraft: OpenTaskEditDraftViewModel;
  taskDetail: TaskRunDetailPanelViewModel;
}

export interface OpenTasksStartupViewModel {
  loading: boolean;
  error: string | null;
}

export interface OpenTasksSidebarViewModel {
  backendMaintenance?: BackendMaintenanceViewModel;
}

export interface OpenTasksHeaderViewModel {
  totalOpenTasks: number;
  projectCount: number;
  worktreeCount: number;
  busy: boolean;
}

export interface OpenTasksRepoSetupViewModel {
  form: RepoSetupFormViewModel;
  discoveredRepos: DiscoveredRepoOption[];
  addBusy: boolean;
  scanBusy: boolean;
  available: boolean;
  scanAvailable: boolean;
}

export interface OpenTasksComposerViewModel {
  form: TaskComposerFormViewModel;
  projects: TaskComposerProjectOption[];
  worktrees: TaskComposerWorktreeOption[];
  busy: boolean;
  canCreate: boolean;
}

export interface OpenTasksScreenActions {
  retryStartup(): void;
  checkBackend(): void;
  reloadDashboard(): void;
  refreshAppRuntime(): void;
  dismissStaleNotice(): void;
  changeRepoSetup(form: RepoSetupFormViewModel): void;
  submitRepoSetup(event: FormEvent<HTMLFormElement>): void;
  scanRepos(event: FormEvent<HTMLFormElement>): void;
  addDiscoveredRepo(repoRootPath: string): void;
  submitComposer(event: FormEvent<HTMLFormElement>): void;
  selectComposerProject(projectId: string): void;
  selectComposerWorktree(worktreeId: string): void;
  changeComposer(patch: Partial<TaskComposerFormViewModel>): void;
  changeEditDraft(draft: OpenTaskEditDraftViewModel): void;
  saveEdit(taskId: OpenTaskId): void;
  cancelEdit(): void;
  changePrompt(taskId: OpenTaskId, prompt: string): void;
  startRun(task: OpenTaskRunTaskViewModel): void;
  updateAttention(taskId: OpenTaskId, attentionState: OpenTaskAttentionValue): void;
  updateExecution(taskId: OpenTaskId, executionState: OpenTaskExecutionValue): void;
  openDetail(taskId: OpenTaskId): void;
  editTask(task: OpenTaskCardViewModel): void;
  archiveTask(taskId: OpenTaskId): void;
  closeTaskDetail(): void;
  reloadTaskDetail(): void;
}
