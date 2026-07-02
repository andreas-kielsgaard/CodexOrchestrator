import { projectOpenTaskDashboard, type DashboardGroup } from './dashboardProjection';
import type { DomainRecords } from './model';

export interface OpenTaskDashboardStore {
  loadOpenTaskDashboardRecords(): Promise<DomainRecords>;
}

export async function loadOpenTaskDashboard(
  store: OpenTaskDashboardStore,
): Promise<DashboardGroup[]> {
  return projectOpenTaskDashboard(await store.loadOpenTaskDashboardRecords());
}

export class InMemoryOpenTaskDashboardStore implements OpenTaskDashboardStore {
  constructor(private readonly records: DomainRecords) {}

  async loadOpenTaskDashboardRecords(): Promise<DomainRecords> {
    return this.records;
  }
}
