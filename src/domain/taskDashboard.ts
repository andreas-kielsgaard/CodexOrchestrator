import { projectOpenTaskDashboard } from './dashboardProjection';
import { seedDomainRecords } from './seedData';

export type { DashboardGroup, DashboardGroupId, DashboardTask } from './dashboardProjection';

export const dashboardGroups = projectOpenTaskDashboard(seedDomainRecords);
