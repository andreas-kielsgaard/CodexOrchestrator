import { dashboardGroups } from './taskDashboard';

describe('dashboardGroups', () => {
  it('keeps the first dashboard slice aligned to the roadmap groups', () => {
    expect(dashboardGroups.map((group) => group.title)).toEqual([
      'Needs action now',
      'Review / decide',
      'Working',
      'Waiting',
      'Later',
    ]);
  });

  it('ships with placeholder tasks for every open-task group', () => {
    expect(dashboardGroups.every((group) => group.tasks.length > 0)).toBe(true);
  });
});
