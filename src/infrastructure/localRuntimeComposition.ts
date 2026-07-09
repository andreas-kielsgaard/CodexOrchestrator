import { randomUUID } from 'node:crypto';

import type { DiffCollectionService } from '../application/commands/diffCollection';
import type {
  PostRunCaptureCompositionService,
} from '../application/commands/postRunCaptureComposition';
import type {
  RepoRegistryScanClock,
  RepoRegistryScanService,
} from '../application/commands/repoRegistryScan';
import type { RunCompositionService } from '../application/commands/runComposition';
import {
  createStoreBackedTaskDashboardClient,
  type TaskDashboardClient,
} from '../application/commands/taskDashboardClient';
import {
  createStoreBackedTaskRunDetailClient,
  type TaskRunDetailClient,
} from '../application/queries/taskRunDetailClient';
import type { TaskRunLifecycleRecorder } from '../application/commands/taskRunLifecycle';
import type { TaskWorktreeSelectionService } from '../application/commands/taskWorktreeSelection';
import type {
  ValidationCommandRunnerService,
} from '../application/commands/validationCommandRunner';
import type { EntityId, IsoDateTime } from '../domain/model';
import type { RepoSyncPlanIdProvider } from '../domain/repoSyncPlanApplier';
import {
  createCodexRuntime,
  type CodexRuntime,
  type CodexRuntimeOptions,
} from './codex/codexRuntime';
import {
  createLocalGitRuntimeAdapters,
  type LocalGitRuntimeAdapters,
  type LocalGitRuntimeAdaptersOptions,
} from './git/localGitRuntime';
import type { AppSqliteStoreBundle } from './sqlite/appStore';
import {
  openLocalAppSqliteDatabase,
  type LocalAppSqliteDatabase,
  type LocalAppSqliteDatabasePath,
  type OpenLocalAppSqliteDatabaseOptions,
} from './sqlite/localAppDatabase';
import {
  createValidationCommandRuntime,
  type ValidationCommandRuntimeOptions,
} from './validation/validationCommandRuntime';

export type OpenLocalRuntimeDatabase = (
  databasePath: LocalAppSqliteDatabasePath,
  options?: OpenLocalAppSqliteDatabaseOptions,
) => LocalAppSqliteDatabase;

export interface LocalRuntimeCompositionOptions {
  database?: OpenLocalAppSqliteDatabaseOptions;
  openDatabase?: OpenLocalRuntimeDatabase;
  git?: LocalRuntimeGitOptions;
  codex?: LocalRuntimeCodexOptions;
  validation?: LocalRuntimeValidationOptions;
  repoRegistry?: LocalRuntimeRepoRegistryOptions;
}

export interface LocalRuntimeGitOptions extends LocalGitRuntimeAdaptersOptions {
  adapters?: LocalGitRuntimeAdapters;
}

export interface LocalRuntimeCodexOptions extends CodexRuntimeOptions {
  runtime?: CodexRuntime;
}

export interface LocalRuntimeValidationOptions extends ValidationCommandRuntimeOptions {
  runtime?: ValidationCommandRunnerService['runtime'];
}

export interface LocalRuntimeRepoRegistryOptions {
  ids?: RepoSyncPlanIdProvider;
  clock?: RepoRegistryScanClock;
}

export interface LocalRuntimeServices {
  taskDashboardClient: TaskDashboardClient;
  taskRunDetailClient: TaskRunDetailClient;
  taskRunLifecycleRecorder: TaskRunLifecycleRecorder;
  runCompositionService: RunCompositionService;
  postRunCaptureCompositionService: PostRunCaptureCompositionService;
  repoRegistryScanService: RepoRegistryScanService;
  taskWorktreeSelectionService: TaskWorktreeSelectionService;
  diffCollectionService: DiffCollectionService;
  validationCommandRunnerService: ValidationCommandRunnerService;
}

export interface LocalRuntimeServiceComposition {
  database: LocalAppSqliteDatabase;
  stores: AppSqliteStoreBundle;
  runtimes: LocalRuntimeRuntimes;
  services: LocalRuntimeServices;
  close(): void;
  dispose(): void;
}

export interface LocalRuntimeRuntimes {
  git: LocalGitRuntimeAdapters;
  codex: CodexRuntime;
  validation: ValidationCommandRunnerService['runtime'];
}

export function openLocalRuntimeServiceComposition(
  databasePath: LocalAppSqliteDatabasePath,
  options: LocalRuntimeCompositionOptions = {},
): LocalRuntimeServiceComposition {
  const database = (options.openDatabase ?? openLocalAppSqliteDatabase)(
    databasePath,
    options.database,
  );
  const stores = database.stores;
  const git = options.git?.adapters ?? createLocalGitRuntimeAdapters(options.git);
  const codex = options.codex?.runtime ?? createCodexRuntime(options.codex);
  const validation =
    options.validation?.runtime ?? createValidationCommandRuntime(options.validation);
  const repoRegistryProviders = createLocalRuntimeRepoRegistryProviders(options.repoRegistry);
  const taskRunLifecycleRecorder = createTaskRunLifecycleRecorder(stores);
  const repoRegistryScanService: RepoRegistryScanService = {
    scanner: git.repoScanner,
    store: stores.repoSync,
    ids: repoRegistryProviders.ids,
    clock: repoRegistryProviders.clock,
  };
  const runCompositionService: RunCompositionService = {
    recorder: taskRunLifecycleRecorder,
    runtime: codex,
  };
  const diffCollectionService: DiffCollectionService = {
    dashboardStore: stores.openTaskDashboard,
    artifactStore: stores.artifact,
    eventStore: stores.event,
    diffProvider: git.diffProvider,
  };
  const validationCommandRunnerService: ValidationCommandRunnerService = {
    dashboardStore: stores.openTaskDashboard,
    validationRunStore: stores.validationRun,
    artifactStore: stores.artifact,
    eventStore: stores.event,
    runtime: validation,
  };
  const services: LocalRuntimeServices = {
    taskDashboardClient: createStoreBackedTaskDashboardClient({
      dashboard: stores.openTaskDashboard,
      write: stores.openTaskWrite,
    }),
    taskRunDetailClient: createStoreBackedTaskRunDetailClient({
      dashboard: stores.openTaskDashboard,
      taskRun: stores.taskRun,
      artifact: stores.artifact,
      event: stores.event,
      validationRun: stores.validationRun,
    }),
    taskRunLifecycleRecorder,
    runCompositionService,
    postRunCaptureCompositionService: {
      runCompositionService,
      diffCollectionService,
      validationCommandRunnerService,
    },
    repoRegistryScanService,
    taskWorktreeSelectionService: {
      dashboardStore: stores.openTaskDashboard,
      taskWriteStore: stores.openTaskWrite,
      repoRegistry: repoRegistryScanService,
      worktreeCreator: git.worktreeCreator,
    },
    diffCollectionService,
    validationCommandRunnerService,
  };

  return {
    database,
    stores,
    runtimes: {
      git,
      codex,
      validation,
    },
    services,
    close: () => database.close(),
    dispose: () => database.dispose(),
  };
}

export function createDefaultLocalRuntimeRepoRegistryProviders(): Required<LocalRuntimeRepoRegistryOptions> {
  return {
    ids: {
      repoId: () => randomUUID() as EntityId,
      branchId: () => randomUUID() as EntityId,
      worktreeId: () => randomUUID() as EntityId,
    },
    clock: {
      now: () => new Date().toISOString() as IsoDateTime,
    },
  };
}

function createTaskRunLifecycleRecorder(stores: AppSqliteStoreBundle): TaskRunLifecycleRecorder {
  return {
    openTaskDashboardStore: stores.openTaskDashboard,
    openTaskWriteStore: stores.openTaskWrite,
    taskRunStore: stores.taskRun,
    conversationStore: stores.conversation,
    artifactStore: stores.artifact,
    eventStore: stores.event,
  };
}

function createLocalRuntimeRepoRegistryProviders(
  options: LocalRuntimeRepoRegistryOptions = {},
): Required<LocalRuntimeRepoRegistryOptions> {
  const defaults = createDefaultLocalRuntimeRepoRegistryProviders();

  return {
    ids: options.ids ?? defaults.ids,
    clock: options.clock ?? defaults.clock,
  };
}
