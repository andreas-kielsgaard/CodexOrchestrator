import { loadOpenTaskDashboard, InMemoryOpenTaskDashboardStore } from './openTaskDashboardStore';
import { seedDomainRecords } from './seedData';

describe('loadOpenTaskDashboard', () => {
  it('projects records loaded through the read store boundary', async () => {
    const groups = await loadOpenTaskDashboard(
      new InMemoryOpenTaskDashboardStore(seedDomainRecords),
    );

    expect(groups.map((group) => group.id)).toEqual([
      'needs_action_now',
      'review_decide',
      'working',
      'waiting',
      'later',
    ]);
    expect(groups.flatMap((group) => group.tasks).length).toBeGreaterThan(0);
  });
});
