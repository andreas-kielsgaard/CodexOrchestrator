import { dashboardGroups } from './taskDashboard';
import { getDashboardGroupId, projectOpenTaskDashboard } from './dashboardProjection';
import { seedDomainRecords } from './seedData';
import type { DomainRecords, Task } from './model';

const baseTask: Task = {
  id: 'task-test',
  projectId: 'project-orchestrator',
  title: 'Test task',
  summary: 'A task used by projection tests.',
  executionState: 'draft',
  attentionState: 'consider_later',
  conversationIds: [],
  priority: 'normal',
  createdAt: '2026-07-01T10:00:00.000Z',
  updatedAt: '2026-07-01T10:00:00.000Z',
};

const recordsWithTask = (task: Task): DomainRecords => ({
  ...seedDomainRecords,
  tasks: [task],
});

describe('projectOpenTaskDashboard', () => {
  it('keeps the open-task groups aligned to the roadmap', () => {
    expect(dashboardGroups.map((group) => group.title)).toEqual([
      'Needs action now',
      'Review / decide',
      'Working',
      'Waiting',
      'Later',
    ]);
  });

  it('projects seed records into every dashboard group', () => {
    expect(dashboardGroups.every((group) => group.tasks.length > 0)).toBe(true);
  });

  it('keeps completed tasks open when they still need review', () => {
    const groups = projectOpenTaskDashboard(
      recordsWithTask({
        ...baseTask,
        executionState: 'completed',
        attentionState: 'needs_review',
      }),
    );

    expect(groups.find((group) => group.id === 'review_decide')?.tasks).toHaveLength(1);
    expect(groups.find((group) => group.id === 'review_decide')?.tasks[0]).toMatchObject({
      executionState: 'completed',
      attentionState: 'needs_review',
      priority: 'normal',
    });
  });

  it('projects task priority for dashboard edit flows', () => {
    const groups = projectOpenTaskDashboard(
      recordsWithTask({
        ...baseTask,
        priority: 'high',
      }),
    );

    expect(groups.find((group) => group.id === 'later')?.tasks[0]).toMatchObject({
      id: 'task-test',
      priority: 'high',
    });
  });

  it('treats running tasks waiting on an agent as working', () => {
    expect(
      getDashboardGroupId({
        ...baseTask,
        executionState: 'running',
        attentionState: 'waiting_on_agent',
      }),
    ).toBe('working');
  });

  it('places non-running tasks waiting on an agent in waiting', () => {
    expect(
      getDashboardGroupId({
        ...baseTask,
        executionState: 'completed',
        attentionState: 'waiting_on_agent',
      }),
    ).toBe('waiting');
  });

  it('omits archived and abandoned tasks from open task projection', () => {
    const groups = projectOpenTaskDashboard({
      ...seedDomainRecords,
      tasks: [
        { ...baseTask, id: 'task-archived', executionState: 'archived' },
        { ...baseTask, id: 'task-abandoned', executionState: 'abandoned' },
      ],
    });

    expect(groups.flatMap((group) => group.tasks)).toHaveLength(0);
  });
});
