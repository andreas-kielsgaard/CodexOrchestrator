import {
  Activity,
  AlertCircle,
  Archive,
  Check,
  CheckCircle2,
  Clock3,
  Edit3,
  GitBranch,
  Inbox,
  LoaderCircle,
  PauseCircle,
  Play,
  Plus,
  RefreshCw,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useState, type FormEvent } from 'react';
import type { DashboardGroupId, DashboardTask } from '../domain/dashboardProjection';
import type { AttentionState, EntityId, ExecutionState, Task } from '../domain/model';
import {
  emptyTaskDashboardSnapshot,
  type TaskDashboardClient,
  type TaskDashboardSnapshot,
} from '../application/taskDashboardClient';
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';

const groupIcons = {
  needs_action_now: Activity,
  review_decide: CheckCircle2,
  working: GitBranch,
  waiting: Clock3,
  later: PauseCircle,
} satisfies Record<DashboardGroupId, typeof Activity>;

const attentionOptions: Array<{ value: AttentionState; label: string }> = [
  { value: 'needs_action_now', label: 'Needs action' },
  { value: 'needs_review', label: 'Needs review' },
  { value: 'waiting_on_agent', label: 'Waiting on agent' },
  { value: 'waiting_on_external', label: 'Waiting external' },
  { value: 'consider_later', label: 'Later' },
  { value: 'snoozed', label: 'Snoozed' },
  { value: 'reference_only', label: 'Reference' },
];

const executionOptions: Array<{ value: ExecutionState; label: string }> = [
  { value: 'draft', label: 'Draft' },
  { value: 'queued', label: 'Queued' },
  { value: 'running', label: 'Running' },
  { value: 'blocked', label: 'Blocked' },
  { value: 'completed', label: 'Completed' },
  { value: 'failed', label: 'Failed' },
];

const priorityOptions: Array<{ value: Task['priority']; label: string }> = [
  { value: 'low', label: 'Low' },
  { value: 'normal', label: 'Normal' },
  { value: 'high', label: 'High' },
];

interface AppProps {
  taskDashboardClient: TaskDashboardClient;
  runtimeCommandClient: RuntimeCommandClient;
}

interface DraftTaskForm {
  projectId: EntityId;
  title: string;
  summary: string;
  attentionState: AttentionState;
  executionState: ExecutionState;
  priority: Task['priority'];
}

type BusyAction = 'load' | 'create' | `update:${string}` | `archive:${string}` | null;

type TaskRunActionStatus = 'running' | 'completed' | 'failed';

interface TaskRunActionState {
  status: TaskRunActionStatus;
  message: string;
}

const initialCreateForm: DraftTaskForm = {
  projectId: '',
  title: '',
  summary: '',
  attentionState: 'needs_action_now',
  executionState: 'draft',
  priority: 'normal',
};

export function App({ taskDashboardClient, runtimeCommandClient }: AppProps) {
  const [snapshot, setSnapshot] = useState<TaskDashboardSnapshot>(() =>
    emptyTaskDashboardSnapshot(),
  );
  const [createForm, setCreateForm] = useState<DraftTaskForm>(initialCreateForm);
  const [editTaskId, setEditTaskId] = useState<EntityId | null>(null);
  const [editForm, setEditForm] = useState<DraftTaskForm>(initialCreateForm);
  const [runPrompts, setRunPrompts] = useState<Record<EntityId, string>>({});
  const [runActions, setRunActions] = useState<Record<EntityId, TaskRunActionState>>({});
  const [busyAction, setBusyAction] = useState<BusyAction>('load');
  const [error, setError] = useState<string | null>(null);

  const canCreate = snapshot.projects.length > 0 && busyAction === null;

  const applySnapshot = useCallback((nextSnapshot: TaskDashboardSnapshot) => {
    setSnapshot(nextSnapshot);
    setCreateForm((current) => ({
      ...current,
      projectId: current.projectId || nextSnapshot.projects[0]?.id || '',
    }));
  }, []);

  const runClientAction = useCallback(
    async (action: Exclude<BusyAction, null>, write: () => Promise<TaskDashboardSnapshot>) => {
      setBusyAction(action);
      setError(null);

      try {
        applySnapshot(await write());
      } catch (caught) {
        setError(errorMessage(caught));
      } finally {
        setBusyAction(null);
      }
    },
    [applySnapshot],
  );

  const loadDashboard = useCallback(async () => {
    await runClientAction('load', () => taskDashboardClient.loadDashboard());
  }, [runClientAction, taskDashboardClient]);

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  const tasksById = useMemo(() => {
    return new Map(snapshot.groups.flatMap((group) => group.tasks).map((task) => [task.id, task]));
  }, [snapshot.groups]);

  const handleCreate = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const title = createForm.title.trim();
    const summary = createForm.summary.trim();

    if (!title || !summary || !createForm.projectId) {
      return;
    }

    void runClientAction('create', async () => {
      const nextSnapshot = await taskDashboardClient.createTask({
        projectId: createForm.projectId,
        title,
        summary,
        attentionState: createForm.attentionState,
        executionState: createForm.executionState,
        priority: createForm.priority,
      });
      setCreateForm((current) => ({
        ...initialCreateForm,
        projectId: current.projectId,
      }));
      return nextSnapshot;
    });
  };

  const startEdit = (task: DashboardTask) => {
    setEditTaskId(task.id);
    setEditForm({
      projectId: '',
      title: task.title,
      summary: task.summary,
      attentionState: task.attentionState,
      executionState: task.executionState,
      priority: task.priority,
    });
  };

  const cancelEdit = () => {
    setEditTaskId(null);
    setEditForm(initialCreateForm);
  };

  const saveEdit = (taskId: EntityId) => {
    const title = editForm.title.trim();
    const summary = editForm.summary.trim();

    if (!title || !summary) {
      return;
    }

    void runClientAction(`update:${taskId}`, async () => {
      const nextSnapshot = await taskDashboardClient.updateTask(taskId, {
        title,
        summary,
        attentionState: editForm.attentionState,
        executionState: editForm.executionState,
        priority: editForm.priority,
      });
      cancelEdit();
      return nextSnapshot;
    });
  };

  const updateTaskState = (
    taskId: EntityId,
    input: { attentionState?: AttentionState; executionState?: ExecutionState },
  ) => {
    void runClientAction(`update:${taskId}`, () => taskDashboardClient.updateTask(taskId, input));
  };

  const archiveTask = (taskId: EntityId) => {
    void runClientAction(`archive:${taskId}`, () => taskDashboardClient.archiveTask(taskId));
  };

  const updateRunPrompt = (taskId: EntityId, prompt: string) => {
    setRunPrompts((current) => ({ ...current, [taskId]: prompt }));
  };

  const startTaskRun = (task: DashboardTask) => {
    const prompt = (runPrompts[task.id] ?? '').trim();

    if (!task.worktreePath || !prompt) {
      return;
    }

    void (async () => {
      setRunActions((current) => ({
        ...current,
        [task.id]: { status: 'running', message: 'Starting Codex run...' },
      }));
      setError(null);

      try {
        const result = await runtimeCommandClient.startCodexTaskRun({
          taskId: task.id,
          prompt,
          cwd: task.worktreePath,
          conversationTitle: task.title,
          conversationSummary: task.summary,
        });

        setRunActions((current) => ({
          ...current,
          [task.id]: {
            status: result.status,
            message: formatRunResult(result),
          },
        }));

        if (result.status === 'completed') {
          setRunPrompts((current) => ({ ...current, [task.id]: '' }));
        }
      } catch (caught) {
        setRunActions((current) => ({
          ...current,
          [task.id]: {
            status: 'failed',
            message: `Run failed: ${errorMessage(caught)}`,
          },
        }));
      } finally {
        try {
          applySnapshot(await taskDashboardClient.loadDashboard());
        } catch (caught) {
          setError(`Dashboard reload failed: ${errorMessage(caught)}`);
        }
      }
    })();
  };

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand-mark">CO</div>
        <nav>
          <a className="nav-item active" href="#open-tasks">
            <Inbox size={18} aria-hidden="true" />
            Open Tasks
          </a>
          <a className="nav-item" href="#projects">
            <GitBranch size={18} aria-hidden="true" />
            Projects
          </a>
        </nav>
      </aside>

      <section className="workspace" id="open-tasks">
        <header className="topbar">
          <div>
            <p className="eyebrow">Local-first control plane</p>
            <h1>Open Tasks</h1>
          </div>
          <div className="status-strip" aria-label="Dashboard totals">
            <span>{snapshot.totalOpenTasks} open</span>
            <span>{snapshot.projects.length} projects</span>
            <button
              className="icon-button"
              type="button"
              onClick={() => void loadDashboard()}
              disabled={busyAction !== null}
              title="Reload dashboard"
              aria-label="Reload dashboard"
            >
              <RefreshCw size={17} aria-hidden="true" />
            </button>
          </div>
        </header>

        {error && (
          <section className="notice error" role="status">
            <AlertCircle size={18} aria-hidden="true" />
            <span>{error}</span>
          </section>
        )}

        <form className="task-composer" onSubmit={handleCreate} aria-label="Create open task">
          <select
            value={createForm.projectId}
            onChange={(event) => setCreateForm({ ...createForm, projectId: event.target.value })}
            disabled={snapshot.projects.length === 0 || busyAction !== null}
            aria-label="Project"
          >
            {snapshot.projects.length === 0 ? (
              <option value="">No persisted projects</option>
            ) : (
              snapshot.projects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))
            )}
          </select>
          <input
            value={createForm.title}
            onChange={(event) => setCreateForm({ ...createForm, title: event.target.value })}
            disabled={!canCreate}
            placeholder="Task title"
            aria-label="Task title"
          />
          <input
            value={createForm.summary}
            onChange={(event) => setCreateForm({ ...createForm, summary: event.target.value })}
            disabled={!canCreate}
            placeholder="Summary"
            aria-label="Task summary"
          />
          <select
            value={createForm.attentionState}
            onChange={(event) =>
              setCreateForm({
                ...createForm,
                attentionState: event.target.value as AttentionState,
              })
            }
            disabled={!canCreate}
            aria-label="Attention state"
          >
            {attentionOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button
            className="primary-action"
            type="submit"
            disabled={!canCreate || !createForm.title.trim() || !createForm.summary.trim()}
          >
            <Plus size={17} aria-hidden="true" />
            Create
          </button>
        </form>

        <section className="dashboard-grid" aria-label="Open task groups">
          {snapshot.groups.map((group) => {
            const Icon = groupIcons[group.id];

            return (
              <article className="task-column" key={group.id}>
                <header className="column-header">
                  <div className="column-title">
                    <Icon size={18} aria-hidden="true" />
                    <h2>{group.title}</h2>
                  </div>
                  <span className="count">{group.tasks.length}</span>
                </header>

                <div className="task-list">
                  {group.tasks.map((task) => {
                    const isEditing = editTaskId === task.id;
                    const runAction = runActions[task.id];
                    const isRunBusy = runAction?.status === 'running';
                    const isBusy =
                      isRunBusy ||
                      busyAction === `update:${task.id}` ||
                      busyAction === `archive:${task.id}`;

                    return (
                      <section className="task-card" key={task.id}>
                        {isEditing ? (
                          <EditTaskForm
                            form={editForm}
                            busy={isBusy}
                            onChange={setEditForm}
                            onSave={() => saveEdit(task.id)}
                            onCancel={cancelEdit}
                          />
                        ) : (
                          <>
                            <div>
                              <h3>{task.title}</h3>
                              <p>{task.summary}</p>
                            </div>
                            <RunTaskForm
                              task={task}
                              prompt={runPrompts[task.id] ?? ''}
                              runAction={runAction}
                              busy={isRunBusy}
                              onPromptChange={(prompt) => updateRunPrompt(task.id, prompt)}
                              onStart={() => startTaskRun(task)}
                            />
                            <div className="task-controls">
                              <select
                                value={task.attentionState}
                                onChange={(event) =>
                                  updateTaskState(task.id, {
                                    attentionState: event.target.value as AttentionState,
                                  })
                                }
                                disabled={isBusy}
                                aria-label={`Attention state for ${task.title}`}
                              >
                                {attentionOptions.map((option) => (
                                  <option key={option.value} value={option.value}>
                                    {option.label}
                                  </option>
                                ))}
                              </select>
                              <select
                                value={task.executionState}
                                onChange={(event) =>
                                  updateTaskState(task.id, {
                                    executionState: event.target.value as ExecutionState,
                                  })
                                }
                                disabled={isBusy}
                                aria-label={`Execution state for ${task.title}`}
                              >
                                {executionOptions.map((option) => (
                                  <option key={option.value} value={option.value}>
                                    {option.label}
                                  </option>
                                ))}
                              </select>
                              <button
                                className="icon-button"
                                type="button"
                                onClick={() => startEdit(tasksById.get(task.id) ?? task)}
                                disabled={isBusy}
                                title="Edit task"
                                aria-label={`Edit ${task.title}`}
                              >
                                <Edit3 size={16} aria-hidden="true" />
                              </button>
                              <button
                                className="icon-button danger"
                                type="button"
                                onClick={() => archiveTask(task.id)}
                                disabled={isBusy}
                                title="Archive task"
                                aria-label={`Archive ${task.title}`}
                              >
                                <Archive size={16} aria-hidden="true" />
                              </button>
                            </div>
                            <footer>
                              <span>{task.project}</span>
                              <span>{task.priority}</span>
                              <span>{task.executionState}</span>
                              <span>{task.attentionState}</span>
                              {task.repo && <span>{task.repo}</span>}
                              {task.branch && <span>{task.branch}</span>}
                              {task.worktreePath && (
                                <span title={task.worktreePath}>
                                  {compactPath(task.worktreePath)}
                                </span>
                              )}
                            </footer>
                          </>
                        )}
                      </section>
                    );
                  })}
                </div>
              </article>
            );
          })}
        </section>
      </section>
    </main>
  );
}

interface RunTaskFormProps {
  task: DashboardTask;
  prompt: string;
  runAction?: TaskRunActionState;
  busy: boolean;
  onPromptChange(prompt: string): void;
  onStart(): void;
}

function RunTaskForm({ task, prompt, runAction, busy, onPromptChange, onStart }: RunTaskFormProps) {
  const hasWorktree = Boolean(task.worktreePath);
  const canStart = hasWorktree && prompt.trim().length > 0 && !busy;

  return (
    <div className="run-controls">
      <div className="run-command">
        <textarea
          value={prompt}
          onChange={(event) => onPromptChange(event.target.value)}
          disabled={!hasWorktree || busy}
          placeholder={hasWorktree ? 'Codex prompt' : 'Worktree required'}
          aria-label={`Codex prompt for ${task.title}`}
          rows={2}
        />
        <button
          className="icon-button run-button"
          type="button"
          onClick={onStart}
          disabled={!canStart}
          title={hasWorktree ? 'Start Codex run' : 'Task needs a worktree'}
          aria-label={`Start Codex run for ${task.title}`}
        >
          {busy ? (
            <LoaderCircle size={16} aria-hidden="true" />
          ) : (
            <Play size={16} aria-hidden="true" />
          )}
        </button>
      </div>
      {hasWorktree ? (
        runAction && (
          <p className={`run-feedback ${runAction.status}`} role="status">
            {runAction.message}
          </p>
        )
      ) : (
        <p className="run-feedback unavailable" role="status">
          No worktree linked
        </p>
      )}
    </div>
  );
}

interface EditTaskFormProps {
  form: DraftTaskForm;
  busy: boolean;
  onChange(form: DraftTaskForm): void;
  onSave(): void;
  onCancel(): void;
}

function EditTaskForm({ form, busy, onChange, onSave, onCancel }: EditTaskFormProps) {
  return (
    <div className="edit-task-form">
      <input
        value={form.title}
        onChange={(event) => onChange({ ...form, title: event.target.value })}
        disabled={busy}
        aria-label="Edit task title"
      />
      <textarea
        value={form.summary}
        onChange={(event) => onChange({ ...form, summary: event.target.value })}
        disabled={busy}
        aria-label="Edit task summary"
        rows={3}
      />
      <div className="edit-grid">
        <select
          value={form.attentionState}
          onChange={(event) =>
            onChange({ ...form, attentionState: event.target.value as AttentionState })
          }
          disabled={busy}
          aria-label="Edit attention state"
        >
          {attentionOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <select
          value={form.executionState}
          onChange={(event) =>
            onChange({ ...form, executionState: event.target.value as ExecutionState })
          }
          disabled={busy}
          aria-label="Edit execution state"
        >
          {executionOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <select
          value={form.priority}
          onChange={(event) =>
            onChange({ ...form, priority: event.target.value as Task['priority'] })
          }
          disabled={busy}
          aria-label="Edit priority"
        >
          {priorityOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
      <div className="edit-actions">
        <button
          className="icon-button"
          type="button"
          onClick={onSave}
          disabled={busy || !form.title.trim() || !form.summary.trim()}
          title="Save task"
          aria-label="Save task"
        >
          <Check size={16} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          type="button"
          onClick={onCancel}
          disabled={busy}
          title="Cancel edit"
          aria-label="Cancel edit"
        >
          <X size={16} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function formatRunResult(result: StartCodexTaskRunCommandResult): string {
  const parts = [
    `${capitalize(result.status)} run ${result.taskRunId}`,
    `task ${result.task.executionState}`,
  ];

  if (result.taskRun.executionState !== result.task.executionState) {
    parts.push(`run ${result.taskRun.executionState}`);
  }

  if (result.exitCode !== undefined) {
    parts.push(`exit ${result.exitCode}`);
  }

  if (result.error ?? result.statusReason) {
    parts.push(result.error ?? result.statusReason ?? '');
  }

  return parts.filter(Boolean).join(' | ');
}

function compactPath(path: string): string {
  const normalizedPath = path.replaceAll('\\', '/');
  const segments = normalizedPath.split('/').filter(Boolean);

  if (segments.length <= 2) {
    return path;
  }

  return `.../${segments.slice(-2).join('/')}`;
}

function capitalize(value: string): string {
  return `${value.slice(0, 1).toUpperCase()}${value.slice(1)}`;
}
