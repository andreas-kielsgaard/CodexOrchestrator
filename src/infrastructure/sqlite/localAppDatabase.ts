import { randomUUID } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';

import type { EntityId, IsoDateTime } from '../../domain/model';
import {
  createAppSqliteStoreBundle,
  initializeAppSqliteStoreDatabase,
  type AppSqliteDatabase,
  type AppSqliteStoreBundle,
  type AppSqliteStoreBundleProviders,
  type AppSqliteStoreIdProvider,
  type AppSqliteStoreTimeProvider,
  type AppSqliteWriteStoreProviders,
  type InitializeAppSqliteStoreDatabaseOptions,
} from './appStore';

export type LocalAppSqliteDatabasePath = string | URL;

export interface ClosableAppSqliteDatabase extends AppSqliteDatabase {
  close(): unknown;
}

export type OpenAppSqliteDatabaseConnection = (
  databasePath: LocalAppSqliteDatabasePath,
) => ClosableAppSqliteDatabase;

export interface OpenLocalAppSqliteDatabaseOptions {
  initialize?: InitializeAppSqliteStoreDatabaseOptions;
  openConnection?: OpenAppSqliteDatabaseConnection;
  providers?: AppSqliteStoreBundleProviders;
}

export interface LocalAppSqliteDatabase {
  db: ClosableAppSqliteDatabase;
  stores: AppSqliteStoreBundle;
  close(): void;
  dispose(): void;
}

export function openLocalAppSqliteDatabase(
  databasePath: LocalAppSqliteDatabasePath,
  options: OpenLocalAppSqliteDatabaseOptions = {},
): LocalAppSqliteDatabase {
  const db = (options.openConnection ?? openNodeSqliteDatabaseConnection)(databasePath);

  try {
    initializeAppSqliteStoreDatabase(db, options.initialize);
    const stores = createAppSqliteStoreBundle(
      db,
      options.providers ?? createDefaultAppSqliteStoreBundleProviders(),
    );
    const close = createIdempotentClose(db);

    return {
      db,
      stores,
      close,
      dispose: close,
    };
  } catch (error) {
    db.close();
    throw error;
  }
}

export function createDefaultAppSqliteStoreBundleProviders(): AppSqliteStoreBundleProviders {
  const ids: AppSqliteStoreIdProvider = {
    nextId: () => randomUUID() as EntityId,
  };
  const clock: AppSqliteStoreTimeProvider = {
    now: () => new Date().toISOString() as IsoDateTime,
  };

  return {
    openTask: createWriteStoreProviders(ids, clock),
    event: createWriteStoreProviders(ids, clock),
    taskRun: createWriteStoreProviders(ids, clock),
    conversation: createWriteStoreProviders(ids, clock),
    artifact: createWriteStoreProviders(ids, clock),
    validationRun: createWriteStoreProviders(ids, clock),
  };
}

function openNodeSqliteDatabaseConnection(
  databasePath: LocalAppSqliteDatabasePath,
): ClosableAppSqliteDatabase {
  return new DatabaseSync(databasePath);
}

function createWriteStoreProviders(
  ids: AppSqliteStoreIdProvider,
  clock: AppSqliteStoreTimeProvider,
): AppSqliteWriteStoreProviders {
  return { ids, clock };
}

function createIdempotentClose(db: ClosableAppSqliteDatabase): () => void {
  let closed = false;

  return () => {
    if (closed) {
      return;
    }

    closed = true;
    db.close();
  };
}
