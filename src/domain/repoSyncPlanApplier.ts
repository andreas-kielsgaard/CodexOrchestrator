import type { Branch, DomainRecords, EntityId, Repo, Worktree } from './model';
import type {
  BranchPlanRef,
  BranchUpsertPlan,
  RepoPlanRef,
  RepoSyncPlan,
  RepoUpsertPlan,
  StaleWorktreePlan,
  WorktreeUpsertPlan,
} from './repoSyncPlanning';

export interface RepoSyncPlanIdProvider {
  repoId(plan: RepoUpsertPlan): EntityId;
  branchId(plan: BranchUpsertPlan): EntityId;
  worktreeId(plan: WorktreeUpsertPlan): EntityId;
}

export type RepoSyncAppliedRecordKind = 'repo' | 'branch' | 'worktree';
export type RepoSyncAppliedAction = 'insert' | 'update';

export interface RepoSyncAppliedChange {
  kind: RepoSyncAppliedRecordKind;
  action: RepoSyncAppliedAction;
  id: EntityId;
}

export interface RepoSyncStaleWorktreeReport {
  action: 'reported_missing_from_scan';
  worktreeId: EntityId;
  repoId: EntityId;
  path: string;
  reason: StaleWorktreePlan['reason'];
  lastObservedAt?: string;
  plannedAt: string;
}

export interface ApplyRepoSyncPlanResult {
  records: DomainRecords;
  changes: RepoSyncAppliedChange[];
  staleWorktrees: RepoSyncStaleWorktreeReport[];
}

export interface ApplyRepoSyncPlanInput {
  records: DomainRecords;
  plan: RepoSyncPlan;
  ids: RepoSyncPlanIdProvider;
}

export function applyRepoSyncPlan(input: ApplyRepoSyncPlanInput): ApplyRepoSyncPlanResult {
  const repoId = resolveRepoPlanId(input.plan.repo, input.ids);
  const repoRefIds = new Map<string, EntityId>([[repoRefKey(input.plan.repo.ref), repoId]]);
  const branchesByRef = new Map<string, EntityId>();

  const repo = applyRepoUpsert(input.plan.repo, repoId);
  const branches = input.plan.branches.map((branchPlan) => {
    const branchId = resolveBranchPlanId(branchPlan, input.ids);
    branchesByRef.set(branchRefKey(branchPlan.ref), branchId);
    return applyBranchUpsert(
      branchPlan,
      branchId,
      resolveRepoRef(branchPlan.values.repo, repoRefIds),
    );
  });
  const worktrees = input.plan.worktrees.map((worktreePlan) =>
    applyWorktreeUpsert(
      worktreePlan,
      resolveWorktreePlanId(worktreePlan, input.ids),
      resolveRepoRef(worktreePlan.values.repo, repoRefIds),
      resolveOptionalBranchRef(worktreePlan.values.branchRef, branchesByRef),
    ),
  );

  return {
    records: {
      ...input.records,
      repos: upsertRecords(input.records.repos, repo, input.plan.repo.action),
      branches: upsertMany(input.records.branches, branches, input.plan.branches),
      worktrees: upsertMany(input.records.worktrees, worktrees, input.plan.worktrees),
    },
    changes: [
      {
        kind: 'repo',
        action: input.plan.repo.action,
        id: repo.id,
      },
      ...input.plan.branches.map((branchPlan, index) => ({
        kind: 'branch' as const,
        action: branchPlan.action,
        id: branches[index].id,
      })),
      ...input.plan.worktrees.map((worktreePlan, index) => ({
        kind: 'worktree' as const,
        action: worktreePlan.action,
        id: worktrees[index].id,
      })),
    ],
    staleWorktrees: input.plan.staleWorktrees.map((stalePlan) =>
      reportStaleWorktree(stalePlan, repoRefIds),
    ),
  };
}

function applyRepoUpsert(plan: RepoUpsertPlan, id: EntityId): Repo {
  return {
    ...(plan.existing ?? {}),
    id,
    projectId: plan.values.projectId,
    name: plan.values.name,
    rootPath: plan.values.rootPath,
    ...(plan.values.defaultBranch !== undefined
      ? { defaultBranch: plan.values.defaultBranch }
      : optionalField('defaultBranch', plan.existing?.defaultBranch)),
    ...(plan.values.remoteUrl !== undefined
      ? { remoteUrl: plan.values.remoteUrl }
      : optionalField('remoteUrl', plan.existing?.remoteUrl)),
    createdAt: plan.existing?.createdAt ?? requireCreatedAt(plan.values.createdAt, 'repo', id),
    updatedAt: plan.values.updatedAt,
  };
}

function applyBranchUpsert(plan: BranchUpsertPlan, id: EntityId, repoId: EntityId): Branch {
  return {
    ...(plan.existing ?? {}),
    id,
    repoId,
    name: plan.values.name,
    ...(plan.values.baseBranch !== undefined
      ? { baseBranch: plan.values.baseBranch }
      : optionalField('baseBranch', plan.existing?.baseBranch)),
    ...(plan.values.headSha !== undefined
      ? { headSha: plan.values.headSha }
      : optionalField('headSha', plan.existing?.headSha)),
    ...(plan.values.intent !== undefined
      ? { intent: plan.values.intent }
      : optionalField('intent', plan.existing?.intent)),
    createdAt: plan.existing?.createdAt ?? requireCreatedAt(plan.values.createdAt, 'branch', id),
    updatedAt: plan.values.updatedAt,
  };
}

function applyWorktreeUpsert(
  plan: WorktreeUpsertPlan,
  id: EntityId,
  repoId: EntityId,
  branchId: EntityId | undefined,
): Worktree {
  return {
    ...worktreeWithoutClearableFields(plan.existing),
    id,
    repoId,
    ...(branchId === undefined ? {} : { branchId }),
    path: plan.values.path,
    isMain: plan.values.isMain,
    isDirty: plan.values.isDirty,
    ...(plan.values.lockReason === null ? {} : { lockReason: plan.values.lockReason }),
    lastScannedAt: plan.values.lastScannedAt,
    createdAt: plan.existing?.createdAt ?? requireCreatedAt(plan.values.createdAt, 'worktree', id),
    updatedAt: plan.values.updatedAt,
  };
}

function worktreeWithoutClearableFields(existing: Worktree | undefined): Partial<Worktree> {
  if (existing === undefined) {
    return {};
  }

  const copy = { ...existing };
  delete copy.branchId;
  delete copy.lockReason;
  return copy;
}

function upsertMany<T extends { id: EntityId }, P extends { action: RepoSyncAppliedAction }>(
  existing: T[],
  applied: T[],
  plans: P[],
): T[] {
  return applied.reduce(
    (records, record, index) => upsertRecords(records, record, plans[index].action),
    existing,
  );
}

function upsertRecords<T extends { id: EntityId }>(
  existing: T[],
  applied: T,
  action: RepoSyncAppliedAction,
): T[] {
  if (action === 'insert') {
    return [...existing, applied];
  }

  return existing.map((record) => (record.id === applied.id ? applied : record));
}

function resolveRepoPlanId(plan: RepoUpsertPlan, ids: RepoSyncPlanIdProvider): EntityId {
  return plan.existing?.id ?? plan.ref.id ?? ids.repoId(plan);
}

function resolveBranchPlanId(plan: BranchUpsertPlan, ids: RepoSyncPlanIdProvider): EntityId {
  return plan.existing?.id ?? plan.ref.id ?? ids.branchId(plan);
}

function resolveWorktreePlanId(plan: WorktreeUpsertPlan, ids: RepoSyncPlanIdProvider): EntityId {
  return plan.existing?.id ?? ids.worktreeId(plan);
}

function resolveRepoRef(ref: RepoPlanRef, repoRefIds: Map<string, EntityId>): EntityId {
  const id = ref.id ?? repoRefIds.get(repoRefKey(ref));

  if (id === undefined) {
    throw new Error(`Unable to resolve repo plan ref for ${ref.projectId}:${ref.rootPath}`);
  }

  return id;
}

function resolveOptionalBranchRef(
  ref: BranchPlanRef | null,
  branchRefIds: Map<string, EntityId>,
): EntityId | undefined {
  if (ref === null) {
    return undefined;
  }

  const id = ref.id ?? branchRefIds.get(branchRefKey(ref));

  if (id === undefined) {
    throw new Error(`Unable to resolve branch plan ref for ${ref.repo.projectId}:${ref.name}`);
  }

  return id;
}

function reportStaleWorktree(
  plan: StaleWorktreePlan,
  repoRefIds: Map<string, EntityId>,
): RepoSyncStaleWorktreeReport {
  return {
    action: 'reported_missing_from_scan',
    worktreeId: plan.existing.id,
    repoId: resolveRepoRef(plan.repo, repoRefIds),
    path: plan.existing.path,
    reason: plan.reason,
    ...(plan.lastObservedAt ? { lastObservedAt: plan.lastObservedAt } : {}),
    plannedAt: plan.plannedAt,
  };
}

function repoRefKey(ref: RepoPlanRef): string {
  return `${ref.projectId}\u0000${ref.rootPath}`;
}

function branchRefKey(ref: BranchPlanRef): string {
  return `${repoRefKey(ref.repo)}\u0000${ref.name}`;
}

function optionalField<T extends string, V>(name: T, value: V | undefined): Record<T, V> | object {
  return value === undefined ? {} : { [name]: value };
}

function requireCreatedAt(
  value: string | undefined,
  kind: RepoSyncAppliedRecordKind,
  id: EntityId,
): string {
  if (value === undefined) {
    throw new Error(`Cannot insert ${kind} ${id} without createdAt in repo sync plan`);
  }

  return value;
}
