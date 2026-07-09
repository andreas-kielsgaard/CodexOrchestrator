import type { GitRepoScanDomainFacts } from './repoScanFacts';
import type { DomainRecords, EntityId, IsoDateTime } from './model';
import {
  applyRepoSyncPlan,
  type ApplyRepoSyncPlanResult,
  type RepoSyncPlanIdProvider,
} from './repoSyncPlanApplier';
import { planRepoSync, type RepoSyncPlan } from './repoSyncPlanning';

export interface SyncRepoFromScanInput {
  records: DomainRecords;
  projectId: EntityId;
  facts: GitRepoScanDomainFacts;
  plannedAt: IsoDateTime | Date;
  ids: RepoSyncPlanIdProvider;
}

export interface SyncRepoFromScanResult {
  plan: RepoSyncPlan;
  applied: ApplyRepoSyncPlanResult;
}

export function syncRepoFromScan(input: SyncRepoFromScanInput): SyncRepoFromScanResult {
  const plan = planRepoSync({
    records: input.records,
    projectId: input.projectId,
    facts: input.facts,
    plannedAt: input.plannedAt,
  });
  const applied = applyRepoSyncPlan({
    records: input.records,
    plan,
    ids: input.ids,
  });

  return {
    plan,
    applied,
  };
}
