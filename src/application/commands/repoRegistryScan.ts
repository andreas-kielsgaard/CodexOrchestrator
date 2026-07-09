import {
  mapGitRepoScanToDomainFacts,
  type GitRepoScanResult,
  type GitRepoScanner,
} from '../ports/gitRepoScanner';
import type { Branch, EntityId, IsoDateTime, Repo, Worktree } from '../../domain/model';
import type {
  RepoSyncAppliedAction,
  RepoSyncAppliedChange,
  RepoSyncAppliedRecordKind,
  RepoSyncStaleWorktreeReport,
  RepoSyncPlanIdProvider,
} from '../../domain/repoSyncPlanApplier';
import type { RepoSyncStore } from '../../domain/repoSyncStore';
import { syncRepoFromScanWithStore } from '../../domain/repoSyncStore';
import { normalizeDomainPath } from '../../domain/repoSyncPlanning';

export interface RepoRegistryScanService {
  scanner: GitRepoScanner;
  store: RepoSyncStore;
  ids: RepoSyncPlanIdProvider;
  clock: RepoRegistryScanClock;
}

export interface RepoRegistryScanClock {
  now(): IsoDateTime;
}

export interface RegisterAndScanRepoInput {
  projectId: EntityId;
  rootPath: string;
  defaultBranch?: string;
  scannedAt?: IsoDateTime;
}

export interface RegisterAndScanRepoResult {
  repo: Repo;
  branches: Branch[];
  worktrees: Worktree[];
  scan: RepoRegistryScanSummary;
  sync: RepoRegistrySyncSummary;
}

export interface RepoRegistryScanSummary {
  rootPath: string;
  scannedAt: IsoDateTime;
  currentBranch?: string;
  defaultBranch?: string;
  remoteUrl?: string;
  isDirty: boolean;
  branchCount: number;
  worktreeCount: number;
}

export interface RepoRegistrySyncSummary {
  plannedAt: IsoDateTime;
  changes: RepoSyncAppliedChange[];
  changeCounts: RepoRegistryChangeCounts;
  staleWorktrees: RepoSyncStaleWorktreeReport[];
}

export type RepoRegistryChangeCounts = Record<
  RepoSyncAppliedRecordKind,
  Record<RepoSyncAppliedAction, number>
>;

export class RepoRegistrySyncedRepoNotFoundError extends Error {
  constructor(repoId: EntityId) {
    super(`Repo registry scan did not produce synced repo: ${repoId}`);
    this.name = 'RepoRegistrySyncedRepoNotFoundError';
  }
}

export class RepoRegistrySyncedRecordNotFoundError extends Error {
  constructor(kind: RepoSyncAppliedRecordKind, id: EntityId) {
    super(`Repo registry scan did not produce synced ${kind}: ${id}`);
    this.name = 'RepoRegistrySyncedRecordNotFoundError';
  }
}

export async function registerAndScanRepo(
  service: RepoRegistryScanService,
  input: RegisterAndScanRepoInput,
): Promise<RegisterAndScanRepoResult> {
  const requestedScannedAt = input.scannedAt ?? service.clock.now();
  const scan = await service.scanner.scanRepo({
    rootPath: input.rootPath,
    ...(input.defaultBranch === undefined ? {} : { defaultBranch: input.defaultBranch }),
    scannedAt: requestedScannedAt,
  });
  const facts = mapGitRepoScanToDomainFacts(scan);
  const sync = await syncRepoFromScanWithStore({
    store: service.store,
    projectId: input.projectId,
    facts,
    plannedAt: scan.scannedAt,
    ids: service.ids,
  });
  const repoId = requireSyncedRepoId(sync.applied.changes);
  const repo = sync.applied.records.repos.find((candidate) => candidate.id === repoId);

  if (repo === undefined) {
    throw new RepoRegistrySyncedRepoNotFoundError(repoId);
  }

  return {
    repo,
    branches: syncedRecordsByKind(sync.applied.records.branches, sync.applied.changes, 'branch'),
    worktrees: syncedRecordsByKind(
      sync.applied.records.worktrees,
      sync.applied.changes,
      'worktree',
    ),
    scan: summarizeScan(scan),
    sync: {
      plannedAt: sync.plan.plannedAt,
      changes: sync.applied.changes,
      changeCounts: countChanges(sync.applied.changes),
      staleWorktrees: sync.applied.staleWorktrees,
    },
  };
}

function summarizeScan(scan: GitRepoScanResult): RepoRegistryScanSummary {
  const facts = mapGitRepoScanToDomainFacts(scan);

  return {
    rootPath: normalizeDomainPath(scan.rootPath),
    scannedAt: scan.scannedAt,
    ...(scan.currentBranch === undefined ? {} : { currentBranch: scan.currentBranch }),
    ...(facts.repo.defaultBranch === undefined ? {} : { defaultBranch: facts.repo.defaultBranch }),
    ...(facts.repo.remoteUrl === undefined ? {} : { remoteUrl: facts.repo.remoteUrl }),
    isDirty: scan.status.isDirty,
    branchCount: scan.branches.length,
    worktreeCount: scan.worktrees.length,
  };
}

function requireSyncedRepoId(changes: RepoSyncAppliedChange[]): EntityId {
  const repoChange = changes.find((change) => change.kind === 'repo');

  if (repoChange === undefined) {
    throw new Error('Repo registry scan expected repo sync to report a repo change');
  }

  return repoChange.id;
}

function syncedRecordsByKind<T extends { id: EntityId }>(
  records: T[],
  changes: RepoSyncAppliedChange[],
  kind: RepoSyncAppliedRecordKind,
): T[] {
  return changes
    .filter((change) => change.kind === kind)
    .map((change) => {
      const record = records.find((candidate) => candidate.id === change.id);

      if (record === undefined) {
        throw new RepoRegistrySyncedRecordNotFoundError(kind, change.id);
      }

      return record;
    });
}

function countChanges(changes: RepoSyncAppliedChange[]): RepoRegistryChangeCounts {
  const counts = emptyChangeCounts();

  for (const change of changes) {
    counts[change.kind][change.action] += 1;
  }

  return counts;
}

function emptyChangeCounts(): RepoRegistryChangeCounts {
  return {
    repo: { insert: 0, update: 0 },
    branch: { insert: 0, update: 0 },
    worktree: { insert: 0, update: 0 },
  };
}
