import type { GitRepoScanDomainFacts } from '../infrastructure/git/types';
import type { DomainRecords, EntityId, IsoDateTime } from './model';
import type { RepoSyncPlanIdProvider } from './repoSyncPlanApplier';
import { normalizeDomainPath } from './repoSyncPlanning';
import { syncRepoFromScan, type SyncRepoFromScanResult } from './repoSyncService';

export type RepoSyncStorePersistedRecords = Pick<DomainRecords, 'repos' | 'branches' | 'worktrees'>;

export interface RepoSyncStoreLoadInput {
  projectId: EntityId;
  rootPath: string;
}

export interface RepoSyncStorePersistInput {
  records: RepoSyncStorePersistedRecords;
  result: SyncRepoFromScanResult;
}

export interface RepoSyncStore {
  loadRepoSyncRecords(input: RepoSyncStoreLoadInput): Promise<DomainRecords>;
  persistRepoSyncRecords(input: RepoSyncStorePersistInput): Promise<void>;
}

export interface SyncRepoFromScanWithStoreInput {
  store: RepoSyncStore;
  projectId: EntityId;
  facts: GitRepoScanDomainFacts;
  plannedAt: IsoDateTime | Date;
  ids: RepoSyncPlanIdProvider;
}

export async function syncRepoFromScanWithStore(
  input: SyncRepoFromScanWithStoreInput,
): Promise<SyncRepoFromScanResult> {
  const records = await input.store.loadRepoSyncRecords({
    projectId: input.projectId,
    rootPath: normalizeDomainPath(input.facts.repo.rootPath),
  });
  const result = syncRepoFromScan({
    records,
    projectId: input.projectId,
    facts: input.facts,
    plannedAt: input.plannedAt,
    ids: input.ids,
  });

  await input.store.persistRepoSyncRecords({
    records: {
      repos: result.applied.records.repos,
      branches: result.applied.records.branches,
      worktrees: result.applied.records.worktrees,
    },
    result,
  });

  return result;
}

export class InMemoryRepoSyncStore implements RepoSyncStore {
  private records: DomainRecords;
  private loadHistory: RepoSyncStoreLoadInput[] = [];
  private persistHistory: RepoSyncStorePersistInput[] = [];

  constructor(records: DomainRecords) {
    this.records = records;
  }

  async loadRepoSyncRecords(input: RepoSyncStoreLoadInput): Promise<DomainRecords> {
    this.loadHistory.push(input);
    return this.records;
  }

  async persistRepoSyncRecords(input: RepoSyncStorePersistInput): Promise<void> {
    this.persistHistory.push(input);
    this.records = {
      ...this.records,
      repos: input.records.repos,
      branches: input.records.branches,
      worktrees: input.records.worktrees,
    };
  }

  snapshot(): DomainRecords {
    return this.records;
  }

  loadedInputs(): RepoSyncStoreLoadInput[] {
    return this.loadHistory;
  }

  persistedResults(): RepoSyncStorePersistInput[] {
    return this.persistHistory;
  }
}
