import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { App } from './App';
import type { DomainRecords, EntityId, Task } from '../domain/model';
import { projectOpenTaskDashboard } from '../domain/dashboardProjection';
import type {
  CreateTaskDashboardTaskInput,
  TaskDashboardClient,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/taskDashboardClient';

const now = '2026-07-02T12:00:00.000Z';

describe('App open task dashboard', () => {
  it('loads tasks through the injected client and supports create, edit, state change, and archive', async () => {
    const client = new FakeTaskDashboardClient();

    render(<App taskDashboardClient={client} />);

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'New dashboard task' },
    });
    fireEvent.change(screen.getByLabelText('Task summary'), {
      target: { value: 'Created from the dashboard.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));

    expect(await screen.findByText('New dashboard task')).toBeInTheDocument();

    const attentionSelect = screen.getByLabelText('Attention state for New dashboard task');
    fireEvent.change(attentionSelect, { target: { value: 'needs_review' } });

    await waitFor(() => {
      expect(client.findTask('task-2')?.attentionState).toBe('needs_review');
    });

    fireEvent.click(screen.getByLabelText('Edit New dashboard task'));
    fireEvent.change(screen.getByLabelText('Edit task title'), {
      target: { value: 'Edited dashboard task' },
    });
    fireEvent.change(screen.getByLabelText('Edit task summary'), {
      target: { value: 'Updated from the edit controls.' },
    });
    fireEvent.click(screen.getByLabelText('Save task'));

    expect(await screen.findByText('Edited dashboard task')).toBeInTheDocument();
    expect(screen.getByText('Updated from the edit controls.')).toBeInTheDocument();

    const editedTaskCard = screen.getByText('Edited dashboard task').closest('.task-card');
    expect(editedTaskCard).not.toBeNull();

    fireEvent.click(
      within(editedTaskCard as HTMLElement).getByLabelText('Archive Edited dashboard task'),
    );

    await waitFor(() => {
      expect(screen.queryByText('Edited dashboard task')).not.toBeInTheDocument();
    });
    expect(client.findTask('task-2')?.executionState).toBe('archived');
  });

  it('shows backend errors from the injected client without rendering seed tasks', async () => {
    const client: TaskDashboardClient = {
      loadDashboard: async () => {
        throw new Error('Persisted task dashboard backend is not connected.');
      },
      createTask: async () => emptySnapshot(),
      updateTask: async () => emptySnapshot(),
      archiveTask: async () => emptySnapshot(),
    };

    render(<App taskDashboardClient={client} />);

    expect(
      await screen.findByText('Persisted task dashboard backend is not connected.'),
    ).toBeInTheDocument();
    expect(screen.queryByText('Run Codex on onboarding flow')).not.toBeInTheDocument();
  });
});

class FakeTaskDashboardClient implements TaskDashboardClient {
  private records: DomainRecords = {
    projects: [
      {
        id: 'project-1',
        name: 'Codex Orchestrator',
        createdAt: now,
        updatedAt: now,
      },
    ],
    repos: [],
    branches: [],
    worktrees: [],
    conversations: [],
    tasks: [
      {
        id: 'task-1',
        projectId: 'project-1',
        conversationIds: [],
        title: 'Existing task',
        summary: 'Already loaded from persistence.',
        executionState: 'draft',
        attentionState: 'needs_action_now',
        priority: 'normal',
        createdAt: now,
        updatedAt: now,
      },
    ],
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
  };

  private nextTaskIndex = 2;
  private nextTick = 1;

  async loadDashboard(): Promise<TaskDashboardSnapshot> {
    return this.snapshot();
  }

  async createTask(input: CreateTaskDashboardTaskInput): Promise<TaskDashboardSnapshot> {
    this.records = {
      ...this.records,
      tasks: [
        ...this.records.tasks,
        {
          id: `task-${this.nextTaskIndex++}`,
          projectId: input.projectId,
          conversationIds: [],
          title: input.title,
          summary: input.summary,
          executionState: input.executionState ?? 'draft',
          attentionState: input.attentionState ?? 'needs_action_now',
          priority: input.priority ?? 'normal',
          createdAt: this.timestamp(),
          updatedAt: this.timestamp(),
        },
      ],
    };

    return this.snapshot();
  }

  async updateTask(
    taskId: EntityId,
    input: UpdateTaskDashboardTaskInput,
  ): Promise<TaskDashboardSnapshot> {
    this.records = {
      ...this.records,
      tasks: this.records.tasks.map((task) =>
        task.id === taskId ? { ...task, ...input, updatedAt: this.timestamp() } : task,
      ),
    };

    return this.snapshot();
  }

  async archiveTask(taskId: EntityId): Promise<TaskDashboardSnapshot> {
    return this.updateTask(taskId, { executionState: 'archived' });
  }

  findTask(taskId: EntityId): Task | undefined {
    return this.records.tasks.find((task) => task.id === taskId);
  }

  private snapshot(): TaskDashboardSnapshot {
    const groups = projectOpenTaskDashboard(this.records);

    return {
      groups,
      projects: this.records.projects.map((project) => ({ id: project.id, name: project.name })),
      totalOpenTasks: groups.reduce((total, group) => total + group.tasks.length, 0),
    };
  }

  private timestamp(): string {
    return `2026-07-02T12:00:${(this.nextTick++).toString().padStart(2, '0')}.000Z`;
  }
}

function emptySnapshot(): TaskDashboardSnapshot {
  return {
    groups: projectOpenTaskDashboard({
      projects: [],
      repos: [],
      branches: [],
      worktrees: [],
      conversations: [],
      tasks: [],
      taskRuns: [],
      artifacts: [],
      validationRuns: [],
      events: [],
    }),
    projects: [],
    totalOpenTasks: 0,
  };
}
