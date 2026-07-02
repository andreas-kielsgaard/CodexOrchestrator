import type { ArtifactStore } from '../../domain/artifactStore';
import type { ConversationStore } from '../../domain/conversationStore';
import type { EventStore } from '../../domain/eventStore';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import type { IdProvider, OpenTaskWriteStore, TimeProvider } from '../../domain/openTaskWriteStore';
import type { RepoSyncStore } from '../../domain/repoSyncStore';
import type { TaskRunStore } from '../../domain/taskRunStore';
import type { ValidationRunStore } from '../../domain/validationRunStore';
import { SqliteArtifactStore, type ArtifactSqliteDatabase } from './artifactStore';
import { SqliteConversationStore, type ConversationSqliteDatabase } from './conversationStore';
import { SqliteEventStore, type EventSqliteDatabase } from './eventStore';
import {
  applyAppSqliteMigrations,
  enableAppSqliteForeignKeys,
  type ApplyAppSqliteMigrationsOptions,
  type AppSqliteMigrationDatabase,
} from './migrationCoordinator';
import {
  SqliteOpenTaskDashboardStore,
  type OpenTaskDashboardSqliteDatabase,
} from './openTaskDashboardStore';
import { SqliteOpenTaskWriteStore, type OpenTaskWriteSqliteDatabase } from './openTaskWriteStore';
import { SqliteRepoSyncStore, type RepoSyncSqliteDatabase } from './repoSyncStore';
import { SqliteTaskRunStore, type TaskRunSqliteDatabase } from './taskRunStore';
import { SqliteValidationRunStore, type ValidationRunSqliteDatabase } from './validationRunStore';

export interface AppSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface AppSqliteDatabase
  extends
    AppSqliteMigrationDatabase,
    RepoSyncSqliteDatabase,
    OpenTaskDashboardSqliteDatabase,
    OpenTaskWriteSqliteDatabase,
    EventSqliteDatabase,
    TaskRunSqliteDatabase,
    ConversationSqliteDatabase,
    ArtifactSqliteDatabase,
    ValidationRunSqliteDatabase {
  exec(sql: string): unknown;
  prepare(sql: string): AppSqliteStatement;
}

export interface InitializeAppSqliteStoreDatabaseOptions {
  migrations?: ApplyAppSqliteMigrationsOptions;
}

export interface AppSqliteStoreBundleProviders {
  openTask: {
    ids: IdProvider;
    clock: TimeProvider;
  };
  event: {
    ids: IdProvider;
    clock: TimeProvider;
  };
  taskRun: {
    ids: IdProvider;
    clock: TimeProvider;
  };
  conversation: {
    ids: IdProvider;
    clock: TimeProvider;
  };
  artifact: {
    ids: IdProvider;
    clock: TimeProvider;
  };
  validationRun: {
    ids: IdProvider;
    clock: TimeProvider;
  };
}

export interface AppSqliteStoreBundle {
  repoSync: RepoSyncStore;
  openTaskDashboard: OpenTaskDashboardStore;
  openTaskWrite: OpenTaskWriteStore;
  event: EventStore;
  taskRun: TaskRunStore;
  conversation: ConversationStore;
  artifact: ArtifactStore;
  validationRun: ValidationRunStore;
}

export function initializeAppSqliteStoreDatabase(
  db: AppSqliteDatabase,
  options: InitializeAppSqliteStoreDatabaseOptions = {},
): void {
  enableAppSqliteForeignKeys(db);
  applyAppSqliteMigrations(db, options.migrations);
}

export function createAppSqliteStoreBundle(
  db: AppSqliteDatabase,
  providers: AppSqliteStoreBundleProviders,
): AppSqliteStoreBundle {
  return {
    repoSync: new SqliteRepoSyncStore(db),
    openTaskDashboard: new SqliteOpenTaskDashboardStore(db),
    openTaskWrite: new SqliteOpenTaskWriteStore(
      db,
      providers.openTask.ids,
      providers.openTask.clock,
    ),
    event: new SqliteEventStore(db, providers.event.ids, providers.event.clock),
    taskRun: new SqliteTaskRunStore(db, providers.taskRun.ids, providers.taskRun.clock),
    conversation: new SqliteConversationStore(
      db,
      providers.conversation.ids,
      providers.conversation.clock,
    ),
    artifact: new SqliteArtifactStore(db, providers.artifact.ids, providers.artifact.clock),
    validationRun: new SqliteValidationRunStore(
      db,
      providers.validationRun.ids,
      providers.validationRun.clock,
    ),
  };
}
