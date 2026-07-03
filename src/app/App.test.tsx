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
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type {
  TaskRunDetailClient,
  TaskRunDetailSnapshot,
} from '../application/taskRunDetailClient';

const now = '2026-07-02T12:00:00.000Z';
const workerPath = 'C:/Repos/Codex Orchestrator Worktrees/042';

describe('App open task dashboard', () => {
  it('loads tasks through the injected client and supports create, edit, state change, and archive', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

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
  }, 10_000);

  it('preserves existing task priority when editing title and summary', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Edit Existing task'));
    fireEvent.change(screen.getByLabelText('Edit task title'), {
      target: { value: 'Priority preserved task' },
    });
    fireEvent.change(screen.getByLabelText('Edit task summary'), {
      target: { value: 'Editing text should not flatten priority.' },
    });
    fireEvent.click(screen.getByLabelText('Save task'));

    expect(await screen.findByText('Priority preserved task')).toBeInTheDocument();
    expect(client.findTask('task-1')?.priority).toBe('high');
  }, 10_000);

  it('shows backend errors from the injected client without rendering seed tasks', async () => {
    const client: TaskDashboardClient = {
      loadDashboard: async () => {
        throw new Error('Persisted task dashboard backend is not connected.');
      },
      createTask: async () => emptySnapshot(),
      updateTask: async () => emptySnapshot(),
      archiveTask: async () => emptySnapshot(),
    };
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(
      await screen.findByText('Persisted task dashboard backend is not connected.'),
    ).toBeInTheDocument();
    expect(screen.queryByText('Run Codex on onboarding flow')).not.toBeInTheDocument();
  });

  it('starts a Codex run through the injected runtime client and reloads the dashboard', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Codex prompt for Existing task'), {
      target: { value: 'Implement the run controls.' },
    });
    fireEvent.click(screen.getByLabelText('Start Codex run for Existing task'));

    expect(await screen.findByText(/Completed run run-1/)).toBeInTheDocument();
    expect(runtimeClient.inputs).toEqual([
      {
        taskId: 'task-1',
        prompt: 'Implement the run controls.',
        cwd: workerPath,
        conversationTitle: 'Existing task',
        conversationSummary: 'Already loaded from persistence.',
      },
    ]);
    expect(client.loadCount).toBe(2);
    expect(screen.getByLabelText('Codex prompt for Existing task')).toHaveValue('');
  }, 10_000);

  it('shows failed Codex run feedback from the injected runtime result', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient(createFailedRunResult);
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Codex prompt for Existing task'), {
      target: { value: 'Try the risky path.' },
    });
    fireEvent.click(screen.getByLabelText('Start Codex run for Existing task'));

    const feedback = await screen.findByText(/Failed run run-2/);
    expect(feedback).toHaveTextContent('task failed');
    expect(feedback).toHaveTextContent('exit 1');
    expect(feedback).toHaveTextContent('Codex failed');
    expect(screen.getByLabelText('Codex prompt for Existing task')).toHaveValue(
      'Try the risky path.',
    );
  }, 10_000);

  it('keeps run controls unavailable for tasks without a worktree path', async () => {
    const client = new FakeTaskDashboardClient({ withWorktree: false });
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    expect(screen.getByText('No worktree linked')).toBeInTheDocument();
    expect(screen.getByLabelText('Codex prompt for Existing task')).toBeDisabled();
    expect(screen.getByLabelText('Start Codex run for Existing task')).toBeDisabled();

    fireEvent.click(screen.getByLabelText('Start Codex run for Existing task'));

    expect(runtimeClient.inputs).toEqual([]);
  }, 10_000);

  it('opens task run detail with anchors, artifacts, validation, and events', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Open detail for Existing task'));

    expect(await screen.findByText('worker/042-run-controls-ui-shell')).toBeInTheDocument();
    expect(screen.getByText('run-detail-1')).toBeInTheDocument();
    expect(screen.getByText('Final response')).toBeInTheDocument();
    expect(screen.getByText('npm test')).toBeInTheDocument();
    expect(screen.getAllByText('run_completed').length).toBeGreaterThan(0);
    expect(detailClient.inputs).toEqual(['task-1']);
  }, 10_000);

  it('shows detail load errors without clearing the dashboard', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient(undefined, {
      taskId: 'task-1',
      error: new Error('Detail backend is not connected.'),
    });

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Open detail for Existing task'));

    expect(await screen.findByText('Detail backend is not connected.')).toBeInTheDocument();
    expect(screen.getByText('Existing task')).toBeInTheDocument();
  }, 10_000);

  it('loads detail for another selected task', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient({
      'task-1': createTaskDetailSnapshot('task-1', 'Existing task', { runId: 'run-detail-1' }),
      'task-2': createTaskDetailSnapshot('task-2', 'Second task', { runId: 'run-detail-2' }),
    });

    await client.createTask({
      projectId: 'project-1',
      title: 'Second task',
      summary: 'Another persisted task.',
      executionState: 'draft',
      attentionState: 'needs_action_now',
      priority: 'normal',
    });

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Second task')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Open detail for Existing task'));
    expect(await screen.findByText('run-detail-1')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Open detail for Second task'));
    expect(await screen.findByText('run-detail-2')).toBeInTheDocument();
    expect(detailClient.inputs).toEqual(['task-1', 'task-2']);
  }, 10_000);

  it('renders an empty no-run detail state', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient({
      'task-1': createTaskDetailSnapshot('task-1', 'Existing task', { includeRun: false }),
    });

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Open detail for Existing task'));

    expect(await screen.findByText('No runs recorded.')).toBeInTheDocument();
    expect(screen.getAllByText('No artifacts recorded.').length).toBeGreaterThan(0);
    expect(screen.getByText('No events recorded.')).toBeInTheDocument();
  }, 10_000);
});

interface FakeTaskDashboardClientOptions {
  withWorktree?: boolean;
}

class FakeTaskDashboardClient implements TaskDashboardClient {
  private records: DomainRecords;

  private nextTaskIndex = 2;
  private nextTick = 1;
  loadCount = 0;

  constructor(options: FakeTaskDashboardClientOptions = {}) {
    const withWorktree = options.withWorktree ?? true;

    this.records = {
      projects: [
        {
          id: 'project-1',
          name: 'Codex Orchestrator',
          createdAt: now,
          updatedAt: now,
        },
      ],
      repos: withWorktree
        ? [
            {
              id: 'repo-1',
              projectId: 'project-1',
              name: 'Codex Orchestrator',
              rootPath: 'C:/Repos/Codex Orchestrator',
              createdAt: now,
              updatedAt: now,
            },
          ]
        : [],
      branches: withWorktree
        ? [
            {
              id: 'branch-1',
              repoId: 'repo-1',
              name: 'worker/042-run-controls-ui-shell',
              createdAt: now,
              updatedAt: now,
            },
          ]
        : [],
      worktrees: withWorktree
        ? [
            {
              id: 'worktree-1',
              repoId: 'repo-1',
              branchId: 'branch-1',
              path: workerPath,
              isMain: false,
              isDirty: false,
              createdAt: now,
              updatedAt: now,
            },
          ]
        : [],
      conversations: [],
      tasks: [
        {
          id: 'task-1',
          projectId: 'project-1',
          ...(withWorktree
            ? { repoId: 'repo-1', branchId: 'branch-1', worktreeId: 'worktree-1' }
            : {}),
          conversationIds: [],
          title: 'Existing task',
          summary: 'Already loaded from persistence.',
          executionState: 'draft',
          attentionState: 'needs_action_now',
          priority: 'high',
          createdAt: now,
          updatedAt: now,
        },
      ],
      taskRuns: [],
      artifacts: [],
      validationRuns: [],
      events: [],
    };
  }

  async loadDashboard(): Promise<TaskDashboardSnapshot> {
    this.loadCount += 1;
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

class FakeRuntimeCommandClient implements RuntimeCommandClient {
  inputs: StartCodexTaskRunCommandInput[] = [];

  constructor(
    private readonly resultFactory: (
      input: StartCodexTaskRunCommandInput,
    ) => StartCodexTaskRunCommandResult = createCompletedRunResult,
  ) {}

  async startCodexTaskRun(
    input: StartCodexTaskRunCommandInput,
  ): Promise<StartCodexTaskRunCommandResult> {
    this.inputs.push(input);

    return this.resultFactory(input);
  }
}

interface FakeTaskRunDetailClientFailure {
  taskId: EntityId;
  error: Error;
}

class FakeTaskRunDetailClient implements TaskRunDetailClient {
  inputs: EntityId[] = [];
  private readonly snapshots: Record<EntityId, TaskRunDetailSnapshot>;

  constructor(
    snapshots: Record<EntityId, TaskRunDetailSnapshot> = {
      'task-1': createTaskDetailSnapshot('task-1', 'Existing task'),
    },
    private readonly failure?: FakeTaskRunDetailClientFailure,
  ) {
    this.snapshots = snapshots;
  }

  async loadTaskRunDetail(taskId: EntityId): Promise<TaskRunDetailSnapshot> {
    this.inputs.push(taskId);

    if (this.failure?.taskId === taskId) {
      throw this.failure.error;
    }

    const snapshot = this.snapshots[taskId];

    if (!snapshot) {
      throw new Error(`Missing detail for ${taskId}`);
    }

    return snapshot;
  }
}

interface CreateTaskDetailSnapshotOptions {
  runId?: EntityId;
  includeRun?: boolean;
}

function createTaskDetailSnapshot(
  taskId: EntityId,
  title: string,
  options: CreateTaskDetailSnapshotOptions = {},
): TaskRunDetailSnapshot {
  const includeRun = options.includeRun ?? true;
  const runId = options.runId ?? 'run-detail-1';

  return {
    task: {
      record: {
        id: taskId,
        projectId: 'project-1',
        repoId: 'repo-1',
        branchId: 'branch-1',
        worktreeId: 'worktree-1',
        conversationIds: [],
        title,
        summary: 'Detail summary.',
        executionState: includeRun ? 'completed' : 'draft',
        attentionState: includeRun ? 'needs_review' : 'needs_action_now',
        priority: 'high',
        createdAt: now,
        updatedAt: now,
      },
      project: {
        id: 'project-1',
        name: 'Codex Orchestrator',
        createdAt: now,
        updatedAt: now,
      },
      repo: {
        id: 'repo-1',
        projectId: 'project-1',
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        createdAt: now,
        updatedAt: now,
      },
      branch: {
        id: 'branch-1',
        repoId: 'repo-1',
        name: 'worker/042-run-controls-ui-shell',
        createdAt: now,
        updatedAt: now,
      },
      worktree: {
        id: 'worktree-1',
        repoId: 'repo-1',
        branchId: 'branch-1',
        path: workerPath,
        isMain: false,
        isDirty: false,
        createdAt: now,
        updatedAt: now,
      },
    },
    runs: includeRun
      ? [
          {
            run: {
              id: runId,
              taskId,
              conversationId: 'conversation-1',
              worktreeId: 'worktree-1',
              executionState: 'completed',
              startedAt: '2026-07-02T12:05:00.000Z',
              completedAt: '2026-07-02T12:06:00.000Z',
              exitCode: 0,
              createdAt: '2026-07-02T12:04:00.000Z',
              updatedAt: '2026-07-02T12:06:00.000Z',
            },
            artifacts: {
              finalResponses: [
                {
                  id: 'artifact-final',
                  taskId,
                  taskRunId: runId,
                  kind: 'final_response',
                  title: 'Final response',
                  content: 'Implemented the detail shell.',
                  createdAt: '2026-07-02T12:06:00.000Z',
                },
              ],
              rawEventStreams: [
                {
                  id: 'artifact-raw',
                  taskId,
                  taskRunId: runId,
                  kind: 'raw_event_stream',
                  title: 'Raw JSONL',
                  content: '{"type":"turn.completed"}',
                  createdAt: '2026-07-02T12:05:30.000Z',
                },
              ],
              diffs: [],
              validationLogs: [
                {
                  id: 'artifact-validation',
                  taskId,
                  taskRunId: runId,
                  kind: 'validation_log',
                  title: 'Validation log',
                  content: 'all green',
                  createdAt: '2026-07-02T12:07:00.000Z',
                },
              ],
              notes: [],
              screenshots: [],
              handoffs: [],
              summaries: [],
              other: [],
            },
            validationRuns: [
              {
                run: {
                  id: 'validation-1',
                  taskId,
                  taskRunId: runId,
                  command: 'npm test',
                  status: 'passed',
                  startedAt: '2026-07-02T12:06:30.000Z',
                  completedAt: '2026-07-02T12:07:00.000Z',
                  exitCode: 0,
                  outputArtifactId: 'artifact-validation',
                  createdAt: '2026-07-02T12:06:30.000Z',
                  updatedAt: '2026-07-02T12:07:00.000Z',
                },
                outputArtifact: {
                  id: 'artifact-validation',
                  taskId,
                  taskRunId: runId,
                  kind: 'validation_log',
                  title: 'Validation log',
                  content: 'all green',
                  createdAt: '2026-07-02T12:07:00.000Z',
                },
              },
            ],
            events: [
              {
                id: 'event-run-completed',
                taskId,
                taskRunId: runId,
                kind: 'run_completed',
                occurredAt: '2026-07-02T12:06:00.000Z',
                payload: { exitCode: 0 },
              },
            ],
          },
        ]
      : [],
    unlinkedArtifacts: emptyArtifactGroups(),
    unlinkedValidationRuns: [],
    eventTimeline: includeRun
      ? [
          {
            id: 'event-run-started',
            taskId,
            taskRunId: runId,
            kind: 'run_started',
            occurredAt: '2026-07-02T12:05:00.000Z',
            payload: { cwd: workerPath },
          },
          {
            id: 'event-run-completed',
            taskId,
            taskRunId: runId,
            kind: 'run_completed',
            occurredAt: '2026-07-02T12:06:00.000Z',
            payload: { exitCode: 0 },
          },
        ]
      : [],
  };
}

function emptyArtifactGroups(): TaskRunDetailSnapshot['unlinkedArtifacts'] {
  return {
    finalResponses: [],
    rawEventStreams: [],
    diffs: [],
    validationLogs: [],
    notes: [],
    screenshots: [],
    handoffs: [],
    summaries: [],
    other: [],
  };
}

function createCompletedRunResult(
  input: StartCodexTaskRunCommandInput,
): StartCodexTaskRunCommandResult {
  return {
    status: 'completed',
    taskId: input.taskId,
    taskRunId: 'run-1',
    conversationId: 'conversation-1',
    rawEventStreamArtifactId: 'artifact-raw',
    finalResponseArtifactId: 'artifact-final',
    exitCode: 0,
    statusReason: 'Codex completed',
    task: {
      id: input.taskId,
      executionState: 'completed',
      attentionState: 'needs_review',
      conversationIds: ['conversation-1'],
      updatedAt: '2026-07-03T10:00:00.000Z',
    },
    taskRun: {
      id: 'run-1',
      executionState: 'completed',
      conversationId: 'conversation-1',
      completedAt: '2026-07-03T10:00:00.000Z',
      exitCode: 0,
      updatedAt: '2026-07-03T10:00:00.000Z',
    },
  };
}

function createFailedRunResult(
  input: StartCodexTaskRunCommandInput,
): StartCodexTaskRunCommandResult {
  return {
    status: 'failed',
    taskId: input.taskId,
    taskRunId: 'run-2',
    conversationId: 'conversation-2',
    rawEventStreamArtifactId: 'artifact-raw-failed',
    exitCode: 1,
    statusReason: 'Codex failed',
    task: {
      id: input.taskId,
      executionState: 'failed',
      attentionState: 'needs_action_now',
      conversationIds: ['conversation-2'],
      updatedAt: '2026-07-03T10:05:00.000Z',
    },
    taskRun: {
      id: 'run-2',
      executionState: 'failed',
      conversationId: 'conversation-2',
      completedAt: '2026-07-03T10:05:00.000Z',
      exitCode: 1,
      updatedAt: '2026-07-03T10:05:00.000Z',
    },
  };
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
