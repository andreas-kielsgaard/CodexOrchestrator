import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { App } from './App';
import type { DomainRecords, EntityId, Task } from '../domain/model';
import { projectOpenTaskDashboard } from '../domain/dashboardProjection';
import type {
  CreateTaskDashboardTaskInput,
  DiscoveredTaskRepo,
  RegisterTaskRepoInput,
  TaskDashboardClient,
  TaskDashboardSnapshot,
  UpdateTaskDashboardTaskInput,
} from '../application/taskDashboardClient';
import type {
  RuntimeCommandClient,
  StartAgentSessionCommandInput,
  StartAgentSessionCommandResult,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type {
  RuntimeStatusClient,
  RuntimeStatusSnapshot,
} from '../application/runtimeStatusClient';
import type {
  TaskRunDetailClient,
  TaskRunDetailSnapshot,
} from '../application/taskRunDetailClient';
import type {
  CreateOrchestrationDraftInput,
  OrchestrationBuildPackage,
  OrchestrationClient,
  OrchestrationRegistrySnapshot,
} from '../application/orchestrationClient';
import { createLocalOrchestrationClient } from '../infrastructure/localOrchestrationClient';

const now = '2026-07-02T12:00:00.000Z';
const workerPath = 'C:/Repos/Codex Orchestrator Worktrees/042';

describe('App open task dashboard', () => {
  it('opens the Agent Session View from the tool viewer and sends prompts through the runtime client', async () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'Agent Session View' }));

    expect(await screen.findByRole('heading', { name: 'Agent Session View' })).toBeInTheDocument();
    const promptBox = screen.getByLabelText('Agent prompt');
    fireEvent.change(promptBox, {
      target: { value: 'Explain this codebase' },
    });
    fireEvent.keyDown(promptBox, { key: 'Enter', ctrlKey: true });

    expect(await screen.findByText('Agent output for Explain this codebase')).toBeInTheDocument();
    expect(screen.getByText('Finished turn')).toBeInTheDocument();
    expect(screen.getAllByText('gpt-5.5').length).toBeGreaterThan(0);
    expect(screen.getByText('never')).toBeInTheDocument();
    expect(screen.queryByLabelText('Agent session ID')).not.toBeInTheDocument();
    expect(screen.queryByText('OpenAI Codex v0.130.0-alpha.5')).not.toBeInTheDocument();
    expect(runtimeClient.agentSessionInputs).toEqual([
      {
        prompt: 'Explain this codebase',
        additionalArgs: [
          '--model',
          'gpt-5.5',
          '--sandbox',
          'danger-full-access',
          '--reasoning-effort',
          'high',
        ],
      },
    ]);
  }, 10_000);

  it('keeps prior Agent Session turns visible after sending another prompt', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: 'Agent Session View' }));

    const promptBox = await screen.findByLabelText('Agent prompt');
    fireEvent.change(promptBox, { target: { value: 'First message' } });
    fireEvent.keyDown(promptBox, { key: 'Enter', ctrlKey: true });

    expect(await screen.findByText('Agent output for First message')).toBeInTheDocument();

    fireEvent.change(promptBox, { target: { value: 'Second message' } });
    fireEvent.keyDown(promptBox, { key: 'Enter', ctrlKey: true });

    expect(await screen.findByText('Agent output for Second message')).toBeInTheDocument();
    expect(screen.getByText('Agent output for First message')).toBeInTheDocument();
    expect(screen.getByText('First message')).toBeInTheDocument();
    expect(screen.getByText('Second message')).toBeInTheDocument();
    expect(runtimeClient.agentSessionInputs).toHaveLength(2);
    expect(runtimeClient.agentSessionInputs[1]).toMatchObject({
      sessionId: 'agent-session-1',
      prompt: 'Second message',
    });
  }, 10_000);

  it('keeps the submitted Agent Session prompt visible when the CLI run fails', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    runtimeClient.agentSessionResultFactory = (input) => ({
      sessionId: 'agent-session-failed',
      status: 'failed',
      command: 'codex',
      args: [
        'exec',
        '--json',
        ...(input.additionalArgs ?? []),
        ...(input.sessionId ? ['resume', input.sessionId] : []),
        input.prompt,
      ],
      stdout: '',
      stderr: 'Something went wrong',
      startedAt: '2026-07-03T10:00:00.000Z',
      completedAt: '2026-07-03T10:01:00.000Z',
      exitCode: 2,
      error: 'Codex session failed with exit code 2',
    });
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Agent Session View' }));

    const promptBox = await screen.findByLabelText('Agent prompt');
    fireEvent.change(promptBox, { target: { value: 'test' } });
    fireEvent.keyDown(promptBox, { key: 'Enter', ctrlKey: true });

    expect(await screen.findByText('test')).toBeInTheDocument();
    expect(await screen.findByText('Failed')).toBeInTheDocument();
    expect(screen.getByText('exit 2')).toBeInTheDocument();
  }, 10_000);

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

    const editNewTaskButton = screen.getByLabelText('Edit New dashboard task');
    await waitFor(() => {
      expect(editNewTaskButton).not.toBeDisabled();
    });
    fireEvent.click(editNewTaskButton);
    fireEvent.change(await screen.findByLabelText('Edit task title'), {
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

  it('shows a startup screen until the dashboard backend responds', async () => {
    const baseClient = new FakeTaskDashboardClient();
    const readySnapshot = await baseClient.loadDashboard();
    let resolveLoad: ((snapshot: TaskDashboardSnapshot) => void) | undefined;
    const client: TaskDashboardClient = {
      loadDashboard: () =>
        new Promise((resolve) => {
          resolveLoad = resolve;
        }),
      createTask: (input) => baseClient.createTask(input),
      updateTask: (taskId, input) => baseClient.updateTask(taskId, input),
      archiveTask: (taskId) => baseClient.archiveTask(taskId),
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

    expect(screen.getByText('Starting local backend')).toBeInTheDocument();

    resolveLoad?.(readySnapshot);

    expect(await screen.findByText('Existing task')).toBeInTheDocument();
    expect(screen.queryByText('Starting local backend')).not.toBeInTheDocument();
  }, 10_000);

  it('shows a retryable startup error when the dashboard backend does not respond', async () => {
    vi.useFakeTimers();
    const baseClient = new FakeTaskDashboardClient();
    const readySnapshot = await baseClient.loadDashboard();
    const pendingLoads: Array<(snapshot: TaskDashboardSnapshot) => void> = [];
    const client: TaskDashboardClient = {
      loadDashboard: vi.fn(
        () =>
          new Promise<TaskDashboardSnapshot>((resolve) => {
            pendingLoads.push(resolve);
          }),
      ),
      createTask: (input) => baseClient.createTask(input),
      updateTask: (taskId, input) => baseClient.updateTask(taskId, input),
      archiveTask: (taskId) => baseClient.archiveTask(taskId),
    };
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    try {
      render(
        <App
          taskDashboardClient={client}
          taskRunDetailClient={detailClient}
          runtimeCommandClient={runtimeClient}
          startupLoadTimeoutMs={25}
        />,
      );

      expect(screen.getByText('Starting local backend')).toBeInTheDocument();

      await act(async () => {
        vi.advanceTimersByTime(25);
        await Promise.resolve();
      });

      expect(screen.getByText('Backend unavailable')).toBeInTheDocument();
      expect(
        screen.getByText(/Dashboard backend did not respond during startup/i),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
      expect(client.loadDashboard).toHaveBeenCalledTimes(2);

      await act(async () => {
        pendingLoads[1]?.(readySnapshot);
        await Promise.resolve();
      });

      expect(screen.getByText('Existing task')).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  }, 10_000);

  it('shows a central refresh widget when the runtime status reports stale backend state', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const reloadApp = vi.fn();
    const runtimeStatusClient = new FakeRuntimeStatusClient({
      available: true,
      stale: true,
      staleTargets: ['backend'],
      reason: 'Rust command changed',
      generation: 'generation-1',
      checkedAt: now,
    });

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        runtimeStatusClient={runtimeStatusClient}
        reloadApp={reloadApp}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();
    expect(screen.getByLabelText('App notifications')).toHaveTextContent('Backend Updated');
    expect(screen.getByLabelText('App notifications')).toHaveTextContent('Rust command changed');

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));

    expect(screen.getByLabelText('App notifications')).toHaveTextContent('Backend Updated');

    fireEvent.click(screen.getByRole('button', { name: 'Refresh App' }));

    await waitFor(() => expect(runtimeStatusClient.clearCount).toBe(1));
    expect(reloadApp).toHaveBeenCalledTimes(1);
  }, 10_000);

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

  it('adds a repo and creates a runnable task against its discovered worktree', async () => {
    const client = new FakeTaskDashboardClient({ empty: true });
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('No persisted projects')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Repo root path'), {
      target: { value: 'C:/Repos/Codex Orchestrator' },
    });
    fireEvent.change(screen.getByLabelText('Project name'), {
      target: { value: 'Codex Orchestrator' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add repo' }));

    expect(await screen.findByText('0 open')).toBeInTheDocument();
    expect(screen.getByLabelText('Worktree')).toHaveValue('worktree-1');

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'Runnable task' },
    });
    fireEvent.change(screen.getByLabelText('Task summary'), {
      target: { value: 'Created after registering a worktree.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));

    expect(await screen.findByText('Runnable task')).toBeInTheDocument();
    expect(screen.getByLabelText('Codex prompt for Runnable task')).not.toBeDisabled();
    expect(client.findTask('task-1')).toMatchObject({
      repoId: 'repo-1',
      branchId: 'branch-1',
      worktreeId: 'worktree-1',
    });
  }, 10_000);

  it('scans a root folder and adds a discovered repo', async () => {
    const client = new FakeTaskDashboardClient({ empty: true });
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
      />,
    );

    expect(await screen.findByText('No persisted projects')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Search root folder'), {
      target: { value: 'C:/Repos' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Scan' }));

    expect(await screen.findByText('Codex Orchestrator')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Codex Orchestrator/i }));

    expect(await screen.findByText('0 open')).toBeInTheDocument();
    expect(screen.getByLabelText('Worktree')).toHaveValue('worktree-1');
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

  it('does not reopen an older task detail when a pending run finishes', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new DeferredRuntimeCommandClient();
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

    fireEvent.change(screen.getByLabelText('Codex prompt for Existing task'), {
      target: { value: 'Run while reviewing.' },
    });
    fireEvent.click(screen.getByLabelText('Start Codex run for Existing task'));

    await waitFor(() => {
      expect(runtimeClient.inputs).toHaveLength(1);
    });

    fireEvent.click(screen.getByLabelText('Open detail for Second task'));
    expect(await screen.findByText('run-detail-2')).toBeInTheDocument();

    runtimeClient.resolve();

    await waitFor(() => {
      expect(client.loadCount).toBe(2);
    });
    expect(detailClient.inputs).toEqual(['task-1', 'task-2']);
    expect(screen.getByText('run-detail-2')).toBeInTheDocument();
    expect(screen.queryByText('run-detail-1')).not.toBeInTheDocument();
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

  it('starts the orchestrations tab on a registry overview', async () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    await waitFor(() =>
      expect(screen.queryByText('Loading orchestration registry')).not.toBeInTheDocument(),
    );

    expect(screen.getByLabelText('Registered orchestrations')).toHaveTextContent(
      'No orchestrations are registered.',
    );
    expect(screen.getAllByRole('button', { name: 'Add Orchestration' }).length).toBeGreaterThan(0);
    expect(screen.queryByText('Agent-OS pinned consumption')).not.toBeInTheDocument();
  }, 10_000);

  it('saves source material as an intake draft without requiring a title first', async () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);

    expect(screen.getByRole('heading', { name: 'Add Orchestration' })).toBeInTheDocument();
    expect(screen.queryByLabelText('Title')).not.toBeInTheDocument();
    expect(screen.queryByText('C:\\Users\\user\\.codex\\orchestrations')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Plan Builder intake stage')).toHaveTextContent('Plan Builder');
    expect(screen.getByLabelText('Plan Builder intake stage')).toHaveTextContent('Not started');
    expect(screen.queryByRole('button', { name: 'Expected Shape' })).not.toBeInTheDocument();

    fireEvent.drop(screen.getByLabelText('Conversation file uploads'), {
      dataTransfer: {
        files: [new File(['handoff'], 'handoff.md', { type: 'text/markdown' })],
      },
    });
    expect(screen.getByLabelText('Uploaded files')).toHaveTextContent('handoff.md');

    fireEvent.change(screen.getByLabelText('Source material'), {
      target: {
        value:
          'Move the target repo to a stable orchestration-ready state.\nKeep planning high-level.',
      },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Plan Builder intake draft' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Runtime unsupported',
    );
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'no Codex thread was created',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'No supported runtime route',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Prompt accepted locally',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Backend integration pending',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Plan builder cannot start because this draft has no explicit linked task/worktree route.',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'handoff.md',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'No artifacts are associated with this conversation.',
    );
    expect(screen.queryByLabelText('Current processing turn')).not.toBeInTheDocument();
    expect(screen.queryByText('Drop files into this conversation')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Plan preview')).not.toBeInTheDocument();
    expect(screen.queryByText('Live State')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Expected Shape' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Open Tasks' }));
    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));

    await waitFor(() => {
      expect(screen.getByLabelText('Registered orchestrations')).toHaveTextContent(
        'Plan Builder intake draft',
      );
    });

    expect(screen.getByLabelText('Registered orchestrations')).toHaveTextContent(
      'Internal storage title; no user title has been set.',
    );
    expect(screen.getByLabelText('Registered orchestrations')).toHaveTextContent('Unsupported');
    expect(screen.getByLabelText('Registered orchestrations')).toHaveTextContent(
      'No runtime thread',
    );

    expect(screen.queryByText('Expected Shape')).not.toBeInTheDocument();
  }, 10_000);

  it('renders orchestration draft state from the injected orchestration client response', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new ControlledOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'User supplied prompt.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Client controlled draft' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Client supplied message.',
    );
    expect(orchestrationClient.createInputs).toEqual([
      {
        title: 'Internal plan-builder intake draft',
        folderPath: 'C:\\Users\\user\\.codex\\orchestrations',
        prompt: 'User supplied prompt.',
        files: [],
      },
    ]);
  }, 10_000);

  it('shows Add Orchestration draft, ready, and submitting states without claiming runtime work', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new DeferredCreateDraftOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);

    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Paste source material or attach files. Plan Builder has not started.',
    );

    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'Source handoff for a local draft.' },
    });
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Source material is ready to save as a local intake draft.',
    );

    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    await waitFor(() => {
      expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
        'Saving source material as an intake draft before requesting Plan Builder runtime start.',
      );
    });
    expect(screen.getByLabelText('Plan builder conversation')).toHaveTextContent(
      'Source handoff for a local draft.',
    );
    expect(screen.getByLabelText('Plan builder conversation')).not.toHaveTextContent(
      'Plan-builder is running',
    );

    await act(async () => {
      await orchestrationClient.resolveCreateDraft();
    });
    expect(
      await screen.findByRole('heading', { name: 'Client controlled draft' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Runtime unsupported',
    );
  }, 10_000);

  it('shows local request-in-flight state after draft save while Plan Builder runtime start is pending', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new DeferredStartPlanBuilderRunOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'Source handoff for a deferred runtime start.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    await waitFor(() => {
      expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
        'Sending runtime request',
      );
    });

    const currentAction = screen.getByLabelText('Add orchestration current action');
    expect(currentAction).toHaveTextContent(
      'Draft saved. Sending Plan Builder runtime request; waiting for backend acknowledgement.',
    );
    expect(currentAction).not.toHaveTextContent('Backend accepted');
    expect(currentAction).not.toHaveTextContent('waiting for the final response');
    expect(currentAction).not.toHaveTextContent('Running');
    expect(currentAction).not.toHaveTextContent('Completed');

    const conversation = screen.getByLabelText('Plan Builder intake draft conversation');
    expect(conversation).toHaveTextContent(
      'Draft saved. Sending Plan Builder runtime request; waiting for backend acknowledgement.',
    );
    expect(conversation).toHaveTextContent('No supported runtime route');
    expect(conversation).not.toHaveTextContent('Backend response evidence');
    expect(conversation).not.toHaveTextContent('Backend accepted');
    expect(conversation).not.toHaveTextContent('waiting for the final response');
    expect(conversation).not.toHaveTextContent('Running');
    expect(conversation).not.toHaveTextContent('Completed');

    expect(orchestrationClient.startInputs).toEqual([{ buildPackageId: 'build-controlled-4' }]);

    await act(async () => {
      await orchestrationClient.resolveStartPlanBuilderRun();
    });

    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Runtime unsupported',
    );
  }, 10_000);

  it('shows a recoverable Add Orchestration failure tied to draft creation', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new FailingCreateDraftOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'This client will fail.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    await waitFor(() => {
      expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
        'Draft creation failed',
      );
    });
    expect(screen.getByRole('status')).toHaveTextContent('Draft service unavailable');
    expect(screen.getByLabelText('Start Plan Builder')).not.toBeDisabled();
    expect(screen.getByLabelText('Plan builder conversation')).toHaveTextContent(
      'This client will fail.',
    );
    expect(screen.getByLabelText('Plan builder conversation')).toHaveTextContent(
      'Draft creation failed. Draft service unavailable',
    );
    expect(screen.getByLabelText('Plan builder conversation')).not.toHaveTextContent(
      'Saving source material as a local intake draft.',
    );
    expect(
      within(screen.getByLabelText('Conversation messages')).getByText('Failed'),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('Current processing turn')).not.toBeInTheDocument();
  }, 10_000);

  it('shows plan-builder running only from a runtime-event-backed client response', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new RuntimeRunningOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    await waitFor(() =>
      expect(screen.queryByText('Loading orchestration registry')).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'A test client will return a runtime event.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Runtime-backed orchestration' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent('Running');
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Plan-builder is running from a runtime event.',
    );
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Runtime event received: plan-builder is running.',
    );
    expect(screen.queryByRole('button', { name: 'Accept Plan Output' })).not.toBeInTheDocument();
  }, 10_000);

  it('shows a Plan Builder review gate, preserves feedback locally, and blocks unsupported instantiation', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new CompletedPlanBuilderOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'Source that produces a plan proposal.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Plan ready for approval' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Plan Builder output is ready for review.',
    );
    expect(
      screen.getByRole('button', { name: 'Confirm build plan and start instantiating' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'Draft plan output with orchestrationPlanDraft JSON.',
    );
    expect(screen.queryByRole('button', { name: 'Expected Shape' })).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Plan Builder feedback'), {
      target: { value: 'Please keep the phases higher-level.' },
    });
    fireEvent.click(screen.getByLabelText('Preserve Plan Builder feedback'));

    await waitFor(() => {
      expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
        'Please keep the phases higher-level.',
      );
    });
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'not sent to the same Plan Builder runtime conversation',
    );

    fireEvent.click(
      screen.getByRole('button', { name: 'Confirm build plan and start instantiating' }),
    );

    await waitFor(() => {
      expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
        'Instantiator runtime unavailable',
      );
    });
    expect(orchestrationClient.stageRequests).toEqual([
      { buildPackageId: 'build-controlled-4', stageId: 'instantiator' },
    ]);
    expect(screen.getByLabelText('Plan Builder intake draft conversation')).toHaveTextContent(
      'No files were generated',
    );
    expect(
      screen.queryByRole('button', { name: 'Confirm build plan and start instantiating' }),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Expected Shape' })).not.toBeInTheDocument();
  }, 10_000);

  it('shows Expected Shape only when instantiator evidence exists', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new InstantiatorEvidenceOrchestrationClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'Source that already has instantiator evidence in this fixture.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Instantiator evidence available' }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Back to Overview' }));
    fireEvent.click(
      screen.getByRole('button', { name: 'Open build package Instantiator evidence available' }),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Expected Shape' }));

    expect(screen.getByLabelText('Expected local plan shape')).toHaveTextContent(
      'orchestration-plan.json',
    );
    expect(screen.getByLabelText('Expected package outputs')).toHaveTextContent(
      'Backend response evidence',
    );
  }, 10_000);

  it('does not infer Start Orchestration from a ready root-startup stage without client action metadata', async () => {
    const client = new FakeTaskDashboardClient();
    const runtimeClient = new FakeRuntimeCommandClient();
    const detailClient = new FakeTaskRunDetailClient();
    const orchestrationClient = new RootStartupReadyWithoutActionClient();

    render(
      <App
        taskDashboardClient={client}
        taskRunDetailClient={detailClient}
        runtimeCommandClient={runtimeClient}
        orchestrationClient={orchestrationClient}
      />,
    );

    expect(await screen.findByText('Existing task')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);
    fireEvent.change(screen.getByLabelText('Source material'), {
      target: { value: 'Client returns root-startup ready without action metadata.' },
    });
    fireEvent.click(screen.getByLabelText('Start Plan Builder'));

    expect(
      await screen.findByRole('heading', { name: 'Root startup action omitted' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent('Ready');
    expect(screen.getByLabelText('Add orchestration current action')).toHaveTextContent(
      'Client snapshot says root startup inputs are ready, but no supported start action was supplied.',
    );
    expect(screen.queryByRole('button', { name: 'Start Orchestration' })).not.toBeInTheDocument();
  }, 10_000);

  it('keeps uploaded files visible in the add-orchestration conversation', async () => {
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

    fireEvent.click(screen.getByRole('button', { name: 'Orchestrations' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Add Orchestration' })[0]);

    fireEvent.change(screen.getByLabelText('Choose conversation files'), {
      target: {
        files: [new File(['source handoff'], 'source-handoff.txt', { type: 'text/plain' })],
      },
    });

    expect(screen.getByLabelText('Uploaded files')).toHaveTextContent('source-handoff.txt');
  }, 10_000);
});

interface FakeTaskDashboardClientOptions {
  empty?: boolean;
  withWorktree?: boolean;
}

class FakeTaskDashboardClient implements TaskDashboardClient {
  private records: DomainRecords;

  private nextTaskIndex = 2;
  private nextProjectIndex = 1;
  private nextRepoIndex = 1;
  private nextBranchIndex = 1;
  private nextWorktreeIndex = 1;
  private nextTick = 1;
  loadCount = 0;

  constructor(options: FakeTaskDashboardClientOptions = {}) {
    const withWorktree = options.withWorktree ?? true;

    this.records = options.empty
      ? {
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
        }
      : {
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

    this.nextTaskIndex = options.empty ? 1 : 2;
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
          ...(input.repoId ? { repoId: input.repoId } : {}),
          ...(input.branchId ? { branchId: input.branchId } : {}),
          ...(input.worktreeId ? { worktreeId: input.worktreeId } : {}),
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

  async registerRepo(input: RegisterTaskRepoInput): Promise<TaskDashboardSnapshot> {
    const repoName = input.repoName ?? 'Codex Orchestrator';
    const projectName = input.projectName ?? repoName;
    const project = this.records.projects.find((record) => record.name === projectName) ?? {
      id: `project-${this.nextProjectIndex++}`,
      name: projectName,
      createdAt: this.timestamp(),
      updatedAt: this.timestamp(),
    };
    const hasProject = this.records.projects.some((record) => record.id === project.id);
    const repo = {
      id: `repo-${this.nextRepoIndex++}`,
      projectId: project.id,
      name: repoName,
      rootPath: input.repoRootPath,
      createdAt: this.timestamp(),
      updatedAt: this.timestamp(),
    };
    const branch = {
      id: `branch-${this.nextBranchIndex++}`,
      repoId: repo.id,
      name: 'worker/042-run-controls-ui-shell',
      createdAt: this.timestamp(),
      updatedAt: this.timestamp(),
    };
    const worktree = {
      id: `worktree-${this.nextWorktreeIndex++}`,
      repoId: repo.id,
      branchId: branch.id,
      path: workerPath,
      isMain: true,
      isDirty: false,
      createdAt: this.timestamp(),
      updatedAt: this.timestamp(),
    };

    this.records = {
      ...this.records,
      projects: hasProject ? this.records.projects : [...this.records.projects, project],
      repos: [...this.records.repos, repo],
      branches: [...this.records.branches, branch],
      worktrees: [...this.records.worktrees, worktree],
    };

    return this.snapshot();
  }

  async discoverRepos(): Promise<DiscoveredTaskRepo[]> {
    return [{ name: 'Codex Orchestrator', path: 'C:/Repos/Codex Orchestrator' }];
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
      repos: this.records.repos.flatMap((repo) => {
        const project = this.records.projects.find((record) => record.id === repo.projectId);

        if (!project) {
          return [];
        }

        return [
          {
            id: repo.id,
            projectId: project.id,
            project: project.name,
            name: repo.name,
            rootPath: repo.rootPath,
          },
        ];
      }),
      worktreeAnchors: this.records.worktrees.flatMap((worktree) => {
        const repo = this.records.repos.find((record) => record.id === worktree.repoId);
        const project = repo
          ? this.records.projects.find((record) => record.id === repo.projectId)
          : undefined;
        const branch = worktree.branchId
          ? this.records.branches.find((record) => record.id === worktree.branchId)
          : undefined;

        if (!repo || !project) {
          return [];
        }

        return [
          {
            id: worktree.id,
            projectId: project.id,
            project: project.name,
            repoId: repo.id,
            repo: repo.name,
            ...(branch ? { branchId: branch.id, branch: branch.name } : {}),
            path: worktree.path,
          },
        ];
      }),
      totalOpenTasks: groups.reduce((total, group) => total + group.tasks.length, 0),
    };
  }

  private timestamp(): string {
    return `2026-07-02T12:00:${(this.nextTick++).toString().padStart(2, '0')}.000Z`;
  }
}

class FakeRuntimeCommandClient implements RuntimeCommandClient {
  inputs: StartCodexTaskRunCommandInput[] = [];
  agentSessionInputs: StartAgentSessionCommandInput[] = [];
  agentSessionResultFactory?: (
    input: StartAgentSessionCommandInput,
  ) => StartAgentSessionCommandResult;

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

  async startAgentSession(
    input: StartAgentSessionCommandInput,
  ): Promise<StartAgentSessionCommandResult> {
    this.agentSessionInputs.push(input);

    if (this.agentSessionResultFactory) {
      return this.agentSessionResultFactory(input);
    }

    return {
      sessionId: 'agent-session-1',
      status: 'completed',
      command: 'codex',
      args: [
        'exec',
        '--json',
        ...(input.additionalArgs ?? []),
        ...(input.sessionId ? ['resume', input.sessionId] : []),
        input.prompt,
      ],
      stdout: [
        JSON.stringify({ type: 'thread.started', thread_id: 'thread-app' }),
        JSON.stringify({ type: 'turn.started' }),
        JSON.stringify({
          type: 'item.completed',
          item: { type: 'agent_message', text: `Agent output for ${input.prompt}` },
        }),
        JSON.stringify({ type: 'turn.completed' }),
      ].join('\n'),
      stderr: [
        'OpenAI Codex v0.130.0-alpha.5',
        'model: gpt-5.5',
        'approval: never',
        'sandbox: danger-full-access',
        'reasoning effort: high',
        'session id: stderr-app-session',
      ].join('\n'),
      startedAt: '2026-07-03T10:00:00.000Z',
      completedAt: '2026-07-03T10:01:00.000Z',
      exitCode: 0,
    };
  }
}

class ControlledOrchestrationClient implements OrchestrationClient {
  private nextId = 1;
  private readonly delegate = createLocalOrchestrationClient({
    now: () => '2026-07-07T10:00:00.000Z',
    nextId: (prefix) => `${prefix}-controlled-${this.nextId++}`,
  });
  private lastCreated: OrchestrationBuildPackage | null = null;

  createInputs: CreateOrchestrationDraftInput[] = [];

  loadOrchestrations(): Promise<OrchestrationRegistrySnapshot> {
    return this.delegate.loadOrchestrations();
  }

  async createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage> {
    this.createInputs.push(input);
    const buildPackage = await this.delegate.createDraft(input);

    const controlledBuild = {
      ...buildPackage,
      title: 'Client controlled draft',
      messages: [
        ...buildPackage.messages,
        {
          id: 'client-controlled-message',
          role: 'system' as const,
          body: 'Client supplied message.',
          createdAt: '2026-07-07T10:00:00.000Z',
          truth: { status: 'integration_pending', provenance: 'unsupported' } as const,
        },
      ],
      stages: buildPackage.stages.map((stage, index) =>
        index === 0
          ? {
              ...stage,
              detail: 'Client supplied current stage detail.',
            }
          : stage,
      ),
    };
    this.lastCreated = controlledBuild;
    return controlledBuild;
  }

  addDraftNote: OrchestrationClient['addDraftNote'] = (input) => this.delegate.addDraftNote(input);

  attachDraftFiles: OrchestrationClient['attachDraftFiles'] = (input) =>
    this.delegate.attachDraftFiles(input);

  requestBuildStage: OrchestrationClient['requestBuildStage'] = (input) =>
    this.delegate.requestBuildStage(input);

  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = await this.delegate.startPlanBuilderRun(input);
    const base = this.lastCreated?.id === buildPackage.id ? this.lastCreated : buildPackage;

    return {
      ...buildPackage,
      title: base.title,
      messages: [
        ...base.messages,
        ...buildPackage.messages.filter(
          (message) => !base.messages.some((candidate) => candidate.id === message.id),
        ),
      ],
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'plan-builder'
          ? {
              ...stage,
              detail: 'Client supplied current stage detail.',
            }
          : stage,
      ),
    };
  }

  startOrchestration: OrchestrationClient['startOrchestration'] = (input) =>
    this.delegate.startOrchestration(input);

  loadOrchestration: OrchestrationClient['loadOrchestration'] = (id) =>
    this.delegate.loadOrchestration(id);

  cancelDraft: OrchestrationClient['cancelDraft'] = (buildPackageId) =>
    this.delegate.cancelDraft(buildPackageId);
}

class DeferredCreateDraftOrchestrationClient extends ControlledOrchestrationClient {
  private resolvePending: (() => Promise<void>) | undefined;

  async createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage> {
    return new Promise((resolve) => {
      this.resolvePending = async () => {
        resolve(await super.createDraft(input));
      };
    });
  }

  async resolveCreateDraft(): Promise<void> {
    if (!this.resolvePending) {
      throw new Error('No deferred orchestration draft request is pending.');
    }

    await this.resolvePending();
    this.resolvePending = undefined;
  }
}

class DeferredStartPlanBuilderRunOrchestrationClient extends ControlledOrchestrationClient {
  startInputs: Array<Parameters<OrchestrationClient['startPlanBuilderRun']>[0]> = [];
  private resolvePending: (() => Promise<void>) | undefined;

  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    this.startInputs.push(input);

    return new Promise((resolve) => {
      this.resolvePending = async () => {
        resolve(await super.startPlanBuilderRun(input));
      };
    });
  }

  async resolveStartPlanBuilderRun(): Promise<void> {
    if (!this.resolvePending) {
      throw new Error('No deferred Plan Builder runtime request is pending.');
    }

    await this.resolvePending();
    this.resolvePending = undefined;
  }
}

class FailingCreateDraftOrchestrationClient extends ControlledOrchestrationClient {
  async createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage> {
    this.createInputs.push(input);
    throw new Error('Draft service unavailable');
  }
}

class RuntimeRunningOrchestrationClient extends ControlledOrchestrationClient {
  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = await super.startPlanBuilderRun(input);
    const runtimeState = { status: 'running', provenance: 'runtime_event' } as const;

    return {
      ...buildPackage,
      clientState: {
        ...buildPackage.clientState,
        status: 'running',
        provenance: 'runtime_event',
        currentAction: 'Plan-builder is running from a runtime event.',
        runtimeSupported: true,
        primaryAction: undefined,
      },
      messages: [
        ...buildPackage.messages,
        {
          id: 'runtime-event-running',
          role: 'system',
          body: 'Runtime event received: plan-builder is running.',
          createdAt: '2026-07-07T10:00:01.000Z',
          truth: runtimeState,
        },
      ],
      stages: buildPackage.stages.map((stage, index) =>
        index === 0
          ? {
              ...stage,
              detail: 'A runtime event confirms plan-builder work is active.',
              state: runtimeState,
              summary: 'Plan-builder running from runtime event evidence.',
            }
          : stage,
      ),
      title: 'Runtime-backed orchestration',
    };
  }
}

class CompletedPlanBuilderOrchestrationClient extends ControlledOrchestrationClient {
  stageRequests: Array<Parameters<OrchestrationClient['requestBuildStage']>[0]> = [];
  protected currentBuild: OrchestrationBuildPackage | null = null;

  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = await super.startPlanBuilderRun(input);
    this.currentBuild = this.withCompletedPlanBuilderEvidence(buildPackage);
    return this.currentBuild;
  }

  addDraftNote: OrchestrationClient['addDraftNote'] = async (input) => {
    const buildPackage = this.requireCurrentBuild(input.buildPackageId);
    const updatedAt = '2026-07-07T10:02:00.000Z';

    this.currentBuild = {
      ...buildPackage,
      updatedAt,
      clientState: {
        ...buildPackage.clientState,
        currentAction:
          'Feedback was preserved locally. Runtime continuation is unsupported for this Plan Builder conversation.',
        notices: [
          {
            id: 'unsupported-plan-builder-continuation',
            kind: 'missing_capability',
            title: 'Runtime continuation unsupported',
            message:
              'Feedback was preserved locally, but it was not sent to the same Plan Builder runtime conversation because continuation is unsupported in this path.',
            truth: { status: 'integration_pending', provenance: 'unsupported' },
          },
        ],
      },
      messages: [
        ...buildPackage.messages,
        {
          id: 'feedback-user',
          role: 'user',
          body: input.body,
          createdAt: updatedAt,
          state: 'completed',
          truth: { status: 'draft', provenance: 'local_draft' },
        },
        {
          id: 'feedback-unsupported',
          role: 'system',
          body: 'Feedback was preserved locally, but it was not sent to the same Plan Builder runtime conversation because continuation is unsupported in this path.',
          createdAt: updatedAt,
          state: 'completed',
          truth: { status: 'integration_pending', provenance: 'unsupported' },
        },
      ],
    };

    return this.currentBuild;
  };

  requestBuildStage: OrchestrationClient['requestBuildStage'] = async (input) => {
    this.stageRequests.push(input);
    const buildPackage = this.requireCurrentBuild(input.buildPackageId);
    const updatedAt = '2026-07-07T10:03:00.000Z';

    this.currentBuild = {
      ...buildPackage,
      updatedAt,
      clientState: {
        ...buildPackage.clientState,
        status: 'integration_pending',
        provenance: 'unsupported',
        currentAction:
          'Instantiator runtime unavailable. The build plan approval was accepted, but no files were generated.',
        notices: [
          {
            id: 'missing-instantiator-runtime',
            kind: 'missing_capability',
            title: 'Instantiator runtime unavailable',
            message:
              'The build plan approval was accepted, but instantiation cannot start because no instantiator runtime route is implemented. No files were generated.',
            truth: { status: 'integration_pending', provenance: 'unsupported' },
          },
        ],
        primaryAction: undefined,
      },
      messages: [
        ...buildPackage.messages,
        {
          id: 'instantiator-unsupported',
          role: 'system',
          body: 'The build plan approval was accepted, but instantiation cannot start because no instantiator runtime route is implemented. No files were generated.',
          createdAt: updatedAt,
          state: 'completed',
          truth: { status: 'integration_pending', provenance: 'unsupported' },
        },
      ],
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'plan-review'
          ? {
              ...stage,
              state: { status: 'completed', provenance: 'backend_response' },
              summary: 'The user confirmed the Plan Builder proposal.',
              detail:
                'Approval was accepted before attempting instantiation. No instantiator runtime route has started.',
            }
          : stage.id === 'instantiator'
            ? {
                ...stage,
                state: { status: 'integration_pending', provenance: 'unsupported' },
                summary: 'Build plan approval was accepted; instantiator runtime is unsupported.',
                detail:
                  'The build plan approval was accepted, but instantiation cannot start because no instantiator runtime route is implemented. No files were generated.',
              }
            : stage,
      ),
    };

    return this.currentBuild;
  };

  protected withCompletedPlanBuilderEvidence(
    buildPackage: OrchestrationBuildPackage,
  ): OrchestrationBuildPackage {
    const completedState = { status: 'completed', provenance: 'backend_response' } as const;
    const readyState = { status: 'ready', provenance: 'backend_response' } as const;

    return {
      ...buildPackage,
      title: 'Plan ready for approval',
      clientState: {
        ...buildPackage.clientState,
        status: 'completed',
        provenance: 'backend_response',
        currentAction:
          'Plan Builder output is ready for review. Confirm the build plan to request instantiation, or preserve feedback locally; runtime continuation is unsupported.',
        runtimeSupported: true,
        primaryAction: {
          id: 'start-instantiation',
          label: 'Confirm build plan and start instantiating',
          enabled: true,
        },
      },
      messages: [
        ...buildPackage.messages,
        {
          id: 'plan-builder-final',
          role: 'assistant',
          body: 'Draft plan output with orchestrationPlanDraft JSON.',
          createdAt: '2026-07-07T10:01:00.000Z',
          state: 'completed',
          truth: completedState,
        },
      ],
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'plan-builder'
          ? {
              ...stage,
              state: completedState,
              summary: 'Plan-builder output is available.',
              detail: 'This is a proposal awaiting explicit user approval.',
            }
          : stage.id === 'plan-review'
            ? {
                ...stage,
                state: readyState,
                summary: 'Plan-builder proposal is ready for user review.',
                detail:
                  'Review the final Plan Builder response. Instantiation starts only after explicit user approval.',
              }
            : stage,
      ),
      stageRuns: [
        {
          id: 'stage-run-plan-builder',
          buildPackageId: buildPackage.id,
          stageId: 'plan-builder',
          state: completedState,
          promptArtifactId: 'artifact-prompt',
          rawEventArtifactId: 'artifact-raw',
          outputArtifactId: 'artifact-final',
          conversationId: 'conversation-plan-builder',
          eventIds: ['event-started', 'event-completed'],
          evidence: {
            schema: 'orchestration-stage-run-evidence/v1',
            externalThreadId: 'thread-plan-builder',
          },
          startedAt: '2026-07-07T10:00:00.000Z',
          completedAt: '2026-07-07T10:01:00.000Z',
          createdAt: '2026-07-07T10:00:00.000Z',
          updatedAt: '2026-07-07T10:01:00.000Z',
        },
      ],
      generatedFiles: [],
    };
  }

  protected requireCurrentBuild(buildPackageId: EntityId): OrchestrationBuildPackage {
    if (!this.currentBuild || this.currentBuild.id !== buildPackageId) {
      throw new Error(`Unknown completed build fixture: ${buildPackageId}`);
    }

    return this.currentBuild;
  }
}

class InstantiatorEvidenceOrchestrationClient extends CompletedPlanBuilderOrchestrationClient {
  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = await super.startPlanBuilderRun(input);
    const completedState = { status: 'completed', provenance: 'backend_response' } as const;

    this.currentBuild = {
      ...buildPackage,
      title: 'Instantiator evidence available',
      clientState: {
        ...buildPackage.clientState,
        currentAction:
          'Instantiator output exists with backend evidence; Expected Shape can be inspected.',
        primaryAction: undefined,
      },
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'plan-review' || stage.id === 'instantiator'
          ? {
              ...stage,
              state: completedState,
              summary: `${stage.title} completed with backend evidence.`,
              detail: `${stage.title} completion is backed by stage-run evidence.`,
            }
          : stage,
      ),
      stageRuns: [
        ...(buildPackage.stageRuns ?? []),
        {
          id: 'stage-run-instantiator',
          buildPackageId: buildPackage.id,
          stageId: 'instantiator',
          state: completedState,
          outputArtifactId: 'artifact-instantiator-final',
          conversationId: 'conversation-plan-builder',
          eventIds: ['event-instantiator-completed'],
          evidence: {
            schema: 'orchestration-stage-run-evidence/v1',
            generatedFiles: ['orchestration-plan.json'],
          },
          startedAt: '2026-07-07T10:02:00.000Z',
          completedAt: '2026-07-07T10:03:00.000Z',
          createdAt: '2026-07-07T10:02:00.000Z',
          updatedAt: '2026-07-07T10:03:00.000Z',
        },
      ],
      generatedFiles: [
        {
          name: 'orchestration-plan.json',
          purpose: 'Reported by instantiator stage-run evidence.',
          state: completedState,
        },
      ],
    };

    return this.currentBuild;
  }
}

class RootStartupReadyWithoutActionClient extends ControlledOrchestrationClient {
  async startPlanBuilderRun(
    input: Parameters<OrchestrationClient['startPlanBuilderRun']>[0],
  ): Promise<OrchestrationBuildPackage> {
    const buildPackage = await super.startPlanBuilderRun(input);
    const completedState = { status: 'completed', provenance: 'backend_response' } as const;
    const readyState = { status: 'ready', provenance: 'backend_response' } as const;

    return {
      ...buildPackage,
      clientState: {
        ...buildPackage.clientState,
        status: 'ready',
        provenance: 'backend_response',
        currentAction:
          'Client snapshot says root startup inputs are ready, but no supported start action was supplied.',
        runtimeSupported: true,
        primaryAction: undefined,
      },
      stages: buildPackage.stages.map((stage) =>
        stage.id === 'root-startup'
          ? {
              ...stage,
              detail:
                'Client snapshot says root startup inputs are ready, but no supported start action was supplied.',
              state: readyState,
              summary: 'Root startup readiness came from the client snapshot.',
            }
          : {
              ...stage,
              state: completedState,
              summary: `${stage.title} completed in this injected backend snapshot.`,
            },
      ),
      title: 'Root startup action omitted',
    };
  }
}

class DeferredRuntimeCommandClient implements RuntimeCommandClient {
  inputs: StartCodexTaskRunCommandInput[] = [];
  agentSessionInputs: StartAgentSessionCommandInput[] = [];
  private resolvePending: ((result: StartCodexTaskRunCommandResult) => void) | undefined;

  async startCodexTaskRun(
    input: StartCodexTaskRunCommandInput,
  ): Promise<StartCodexTaskRunCommandResult> {
    this.inputs.push(input);

    return new Promise((resolve) => {
      this.resolvePending = resolve;
    });
  }

  resolve(): void {
    const input = this.inputs[this.inputs.length - 1];

    if (input === undefined || this.resolvePending === undefined) {
      throw new Error('No pending runtime command to resolve.');
    }

    this.resolvePending(createCompletedRunResult(input));
    this.resolvePending = undefined;
  }

  async startAgentSession(
    input: StartAgentSessionCommandInput,
  ): Promise<StartAgentSessionCommandResult> {
    this.agentSessionInputs.push(input);
    const now = '2026-07-03T10:00:00.000Z';

    return {
      sessionId: 'agent-session-deferred',
      status: 'completed',
      command: 'codex',
      args: [
        'exec',
        '--json',
        ...(input.additionalArgs ?? []),
        ...(input.sessionId ? ['resume', input.sessionId] : []),
        input.prompt,
      ],
      stdout: 'Agent output',
      stderr: '',
      startedAt: now,
      completedAt: now,
      exitCode: 0,
    };
  }
}

class FakeRuntimeStatusClient implements RuntimeStatusClient {
  clearCount = 0;

  constructor(private status: RuntimeStatusSnapshot) {}

  async checkStatus(): Promise<RuntimeStatusSnapshot> {
    return this.status;
  }

  async clearStale(): Promise<RuntimeStatusSnapshot> {
    this.clearCount += 1;
    this.status = {
      ...this.status,
      stale: false,
      staleTargets: [],
      generation: 'fresh-generation',
    };
    return this.status;
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
    repos: [],
    worktreeAnchors: [],
    totalOpenTasks: 0,
  };
}
