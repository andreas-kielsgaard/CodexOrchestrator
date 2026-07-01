import type { GitRepoScanDomainFacts } from '../infrastructure/git/types';
import type { Branch, DomainRecords, EntityId, IsoDateTime, Repo, Worktree } from './model';

export type RepoSyncPlanAction = 'insert' | 'update';

export interface RepoPlanRef {
  kind: 'existing' | 'planned';
  projectId: EntityId;
  rootPath: string;
  id?: EntityId;
}

export interface BranchPlanRef {
  kind: 'existing' | 'planned';
  repo: RepoPlanRef;
  name: string;
  id?: EntityId;
}

export interface RepoUpsertPlan {
  action: RepoSyncPlanAction;
  match: {
    projectId: EntityId;
    rootPath: string;
  };
  ref: RepoPlanRef;
  existing?: Repo;
  values: {
    projectId: EntityId;
    name: string;
    rootPath: string;
    defaultBranch?: string;
    remoteUrl?: string;
    updatedAt: IsoDateTime;
    createdAt?: IsoDateTime;
  };
}

export interface BranchUpsertPlan {
  action: RepoSyncPlanAction;
  match: {
    repo: RepoPlanRef;
    name: string;
  };
  ref: BranchPlanRef;
  existing?: Branch;
  values: {
    repo: RepoPlanRef;
    name: string;
    headSha?: string;
    baseBranch?: string;
    intent?: string;
    updatedAt: IsoDateTime;
    createdAt?: IsoDateTime;
  };
}

export interface WorktreeUpsertPlan {
  action: RepoSyncPlanAction;
  match: {
    repo: RepoPlanRef;
    path: string;
  };
  existing?: Worktree;
  branchRef?: BranchPlanRef;
  values: {
    repo: RepoPlanRef;
    path: string;
    isMain: boolean;
    isDirty: boolean;
    lockReason?: string;
    lastScannedAt: IsoDateTime;
    updatedAt: IsoDateTime;
    createdAt?: IsoDateTime;
    branchRef?: BranchPlanRef;
  };
}

export interface StaleWorktreePlan {
  action: 'mark_missing_from_scan';
  existing: Worktree;
  repo: RepoPlanRef;
  reason: 'absent_from_current_git_scan';
  lastObservedAt?: IsoDateTime;
  plannedAt: IsoDateTime;
}

export interface RepoSyncPlan {
  projectId: EntityId;
  plannedAt: IsoDateTime;
  repo: RepoUpsertPlan;
  branches: BranchUpsertPlan[];
  worktrees: WorktreeUpsertPlan[];
  staleWorktrees: StaleWorktreePlan[];
}

export interface PlanRepoSyncInput {
  records: DomainRecords;
  projectId: EntityId;
  facts: GitRepoScanDomainFacts;
  plannedAt: IsoDateTime | Date;
}

export function planRepoSync(input: PlanRepoSyncInput): RepoSyncPlan {
  const plannedAt = isoTimestamp(input.plannedAt);
  const rootPath = normalizeDomainPath(input.facts.repo.rootPath);
  const existingRepo = findExistingRepo(input.records.repos, input.projectId, rootPath);
  const repoRef = repoPlanRef(input.projectId, rootPath, existingRepo);
  const repoPlan = planRepoUpsert(input, plannedAt, rootPath, existingRepo, repoRef);
  const branchPlans = input.facts.branches.map((branchFact) =>
    planBranchUpsert(
      input.records.branches,
      plannedAt,
      branchFact.name,
      branchFact.headSha,
      repoRef,
    ),
  );
  const branchRefsByName = new Map(branchPlans.map((plan) => [plan.ref.name, plan.ref]));
  const worktreePlans = input.facts.worktrees.map((worktreeFact) =>
    planWorktreeUpsert(
      input.records.worktrees,
      plannedAt,
      normalizeDomainPath(worktreeFact.path),
      worktreeFact.isMain,
      worktreeFact.isDirty,
      worktreeFact.lockReason,
      isoTimestamp(worktreeFact.lastScannedAt),
      repoRef,
      worktreeFact.branchName ? branchRefsByName.get(worktreeFact.branchName) : undefined,
    ),
  );
  const scannedWorktreePaths = new Set(worktreePlans.map((plan) => plan.match.path));

  return {
    projectId: input.projectId,
    plannedAt,
    repo: repoPlan,
    branches: branchPlans,
    worktrees: worktreePlans,
    staleWorktrees: existingRepo
      ? input.records.worktrees
          .filter((worktree) => worktree.repoId === existingRepo.id)
          .filter((worktree) => !scannedWorktreePaths.has(normalizeDomainPath(worktree.path)))
          .map((worktree) => ({
            action: 'mark_missing_from_scan',
            existing: worktree,
            repo: repoRef,
            reason: 'absent_from_current_git_scan',
            lastObservedAt: worktree.lastScannedAt,
            plannedAt,
          }))
      : [],
  };
}

export function normalizeDomainPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function planRepoUpsert(
  input: PlanRepoSyncInput,
  plannedAt: IsoDateTime,
  rootPath: string,
  existingRepo: Repo | undefined,
  repoRef: RepoPlanRef,
): RepoUpsertPlan {
  const defaultBranch = input.facts.repo.defaultBranch ?? existingRepo?.defaultBranch;
  const remoteUrl = input.facts.repo.remoteUrl ?? existingRepo?.remoteUrl;

  return {
    action: existingRepo ? 'update' : 'insert',
    match: {
      projectId: input.projectId,
      rootPath,
    },
    ref: repoRef,
    ...(existingRepo ? { existing: existingRepo } : {}),
    values: {
      projectId: input.projectId,
      name: input.facts.repo.name,
      rootPath,
      ...(defaultBranch ? { defaultBranch } : {}),
      ...(remoteUrl ? { remoteUrl } : {}),
      updatedAt: plannedAt,
      ...(existingRepo ? {} : { createdAt: plannedAt }),
    },
  };
}

function planBranchUpsert(
  branches: Branch[],
  plannedAt: IsoDateTime,
  name: string,
  headSha: string | undefined,
  repoRef: RepoPlanRef,
): BranchUpsertPlan {
  const existingBranch =
    repoRef.id === undefined
      ? undefined
      : branches.find((branch) => branch.repoId === repoRef.id && branch.name === name);
  const branchRef: BranchPlanRef = {
    kind: existingBranch ? 'existing' : 'planned',
    repo: repoRef,
    name,
    ...(existingBranch ? { id: existingBranch.id } : {}),
  };

  return {
    action: existingBranch ? 'update' : 'insert',
    match: {
      repo: repoRef,
      name,
    },
    ref: branchRef,
    ...(existingBranch ? { existing: existingBranch } : {}),
    values: {
      repo: repoRef,
      name,
      ...(headSha ? { headSha } : {}),
      ...(existingBranch?.baseBranch ? { baseBranch: existingBranch.baseBranch } : {}),
      ...(existingBranch?.intent ? { intent: existingBranch.intent } : {}),
      updatedAt: plannedAt,
      ...(existingBranch ? {} : { createdAt: plannedAt }),
    },
  };
}

function planWorktreeUpsert(
  worktrees: Worktree[],
  plannedAt: IsoDateTime,
  path: string,
  isMain: boolean,
  isDirty: boolean,
  lockReason: string | undefined,
  lastScannedAt: IsoDateTime,
  repoRef: RepoPlanRef,
  branchRef: BranchPlanRef | undefined,
): WorktreeUpsertPlan {
  const existingWorktree =
    repoRef.id === undefined
      ? undefined
      : worktrees.find(
          (worktree) =>
            worktree.repoId === repoRef.id && normalizeDomainPath(worktree.path) === path,
        );

  return {
    action: existingWorktree ? 'update' : 'insert',
    match: {
      repo: repoRef,
      path,
    },
    ...(existingWorktree ? { existing: existingWorktree } : {}),
    ...(branchRef ? { branchRef } : {}),
    values: {
      repo: repoRef,
      path,
      isMain,
      isDirty,
      ...(lockReason ? { lockReason } : {}),
      lastScannedAt,
      updatedAt: plannedAt,
      ...(existingWorktree ? {} : { createdAt: plannedAt }),
      ...(branchRef ? { branchRef } : {}),
    },
  };
}

function findExistingRepo(repos: Repo[], projectId: EntityId, rootPath: string): Repo | undefined {
  return repos.find(
    (repo) => repo.projectId === projectId && normalizeDomainPath(repo.rootPath) === rootPath,
  );
}

function repoPlanRef(projectId: EntityId, rootPath: string, existingRepo?: Repo): RepoPlanRef {
  return {
    kind: existingRepo ? 'existing' : 'planned',
    projectId,
    rootPath,
    ...(existingRepo ? { id: existingRepo.id } : {}),
  };
}

function isoTimestamp(value: IsoDateTime | Date): IsoDateTime {
  return value instanceof Date ? value.toISOString() : value;
}
