import {
  Activity,
  AlertCircle,
  Archive,
  Check,
  CheckCircle2,
  Clock3,
  FileText,
  Edit3,
  GitBranch,
  Inbox,
  LoaderCircle,
  PauseCircle,
  Play,
  Plus,
  RefreshCw,
  Route,
  ScrollText,
  ShieldCheck,
  X,
  type LucideIcon,
} from 'lucide-react';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from 'react';
import type { DashboardGroupId, DashboardTask } from '../domain/dashboardProjection';
import type {
  Artifact,
  AttentionState,
  EntityId,
  Event,
  ExecutionState,
  Task,
} from '../domain/model';
import {
  emptyTaskDashboardSnapshot,
  type RegisterTaskWorktreeInput,
  type TaskDashboardClient,
  type TaskDashboardSnapshot,
  type TaskDashboardWorktreeAnchor,
} from '../application/taskDashboardClient';
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandResult,
} from '../application/runtimeCommandClient';
import type {
  TaskRunDetailArtifactGroups,
  TaskRunDetailClient,
  TaskRunDetailRun,
  TaskRunDetailSnapshot,
  TaskRunDetailValidationRun,
} from '../application/taskRunDetailClient';

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
  taskRunDetailClient: TaskRunDetailClient;
  runtimeCommandClient: RuntimeCommandClient;
}

interface DraftTaskForm {
  projectId: EntityId;
  worktreeId: EntityId;
  title: string;
  summary: string;
  attentionState: AttentionState;
  executionState: ExecutionState;
  priority: Task['priority'];
}

interface WorktreeSetupForm {
  projectName: string;
  repoName: string;
  repoRootPath: string;
  worktreePath: string;
  branchName: string;
}

type BusyAction =
  'load' | 'register-worktree' | 'create' | `update:${string}` | `archive:${string}` | null;

type TaskRunActionStatus = 'running' | 'completed' | 'failed';
type DetailStatus = 'idle' | 'loading' | 'loaded' | 'failed';

interface TaskRunActionState {
  status: TaskRunActionStatus;
  message: string;
}

interface DetailState {
  taskId: EntityId | null;
  status: DetailStatus;
  snapshot: TaskRunDetailSnapshot | null;
  error: string | null;
}

const initialCreateForm: DraftTaskForm = {
  projectId: '',
  worktreeId: '',
  title: '',
  summary: '',
  attentionState: 'needs_action_now',
  executionState: 'draft',
  priority: 'normal',
};

const initialWorktreeSetupForm: WorktreeSetupForm = {
  projectName: '',
  repoName: '',
  repoRootPath: '',
  worktreePath: '',
  branchName: '',
};

export function App({ taskDashboardClient, taskRunDetailClient, runtimeCommandClient }: AppProps) {
  const [snapshot, setSnapshot] = useState<TaskDashboardSnapshot>(() =>
    emptyTaskDashboardSnapshot(),
  );
  const [worktreeSetupForm, setWorktreeSetupForm] =
    useState<WorktreeSetupForm>(initialWorktreeSetupForm);
  const [createForm, setCreateForm] = useState<DraftTaskForm>(initialCreateForm);
  const [editTaskId, setEditTaskId] = useState<EntityId | null>(null);
  const [editForm, setEditForm] = useState<DraftTaskForm>(initialCreateForm);
  const [runPrompts, setRunPrompts] = useState<Record<EntityId, string>>({});
  const [runActions, setRunActions] = useState<Record<EntityId, TaskRunActionState>>({});
  const [detail, setDetail] = useState<DetailState>({
    taskId: null,
    status: 'idle',
    snapshot: null,
    error: null,
  });
  const detailTaskIdRef = useRef<EntityId | null>(null);
  const [busyAction, setBusyAction] = useState<BusyAction>('load');
  const [error, setError] = useState<string | null>(null);

  const canCreate = snapshot.projects.length > 0 && busyAction === null;

  const applySnapshot = useCallback((nextSnapshot: TaskDashboardSnapshot) => {
    setSnapshot(nextSnapshot);
    setCreateForm((current) => ({
      ...current,
      ...nextCreateFormAnchorDefaults(current, nextSnapshot),
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

  const registerWorktree = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const input = normalizeWorktreeSetupInput(worktreeSetupForm);

    if (!input || !taskDashboardClient.registerWorktree) {
      return;
    }

    void runClientAction('register-worktree', async () => {
      const nextSnapshot = await taskDashboardClient.registerWorktree?.(input);

      if (!nextSnapshot) {
        throw new Error('Repo/worktree registration is not available.');
      }

      setWorktreeSetupForm((current) => ({
        ...current,
        repoRootPath: current.repoRootPath.trim() || input.repoRootPath,
        worktreePath: input.worktreePath,
      }));
      return nextSnapshot;
    });
  };

  const loadTaskDetail = useCallback(
    async (taskId: EntityId) => {
      setDetail((current) => ({
        taskId,
        status: 'loading',
        snapshot: current.taskId === taskId ? current.snapshot : null,
        error: null,
      }));

      try {
        const detailSnapshot = await taskRunDetailClient.loadTaskRunDetail(taskId);
        setDetail({
          taskId,
          status: 'loaded',
          snapshot: detailSnapshot,
          error: null,
        });
      } catch (caught) {
        setDetail({
          taskId,
          status: 'failed',
          snapshot: null,
          error: errorMessage(caught),
        });
      }
    },
    [taskRunDetailClient],
  );

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  useEffect(() => {
    detailTaskIdRef.current = detail.taskId;
  }, [detail.taskId]);

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
      const selectedAnchor = selectedWorktreeAnchor(snapshot, createForm.worktreeId);
      const nextSnapshot = await taskDashboardClient.createTask({
        projectId: selectedAnchor?.projectId ?? createForm.projectId,
        repoId: selectedAnchor?.repoId,
        branchId: selectedAnchor?.branchId,
        worktreeId: selectedAnchor?.id,
        title,
        summary,
        attentionState: createForm.attentionState,
        executionState: createForm.executionState,
        priority: createForm.priority,
      });
      setCreateForm((current) => ({
        ...initialCreateForm,
        projectId: current.projectId,
        worktreeId: current.worktreeId,
      }));
      return nextSnapshot;
    });
  };

  const startEdit = (task: DashboardTask) => {
    setEditTaskId(task.id);
    setEditForm({
      projectId: '',
      worktreeId: '',
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
    setDetail((current) =>
      current.taskId === taskId
        ? { taskId: null, status: 'idle', snapshot: null, error: null }
        : current,
    );
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
          if (detailTaskIdRef.current === task.id) {
            await loadTaskDetail(task.id);
          }
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
            <span>{snapshot.worktreeAnchors.length} worktrees</span>
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

        <RegisterWorktreeForm
          form={worktreeSetupForm}
          busy={busyAction === 'register-worktree'}
          available={Boolean(taskDashboardClient.registerWorktree)}
          onChange={setWorktreeSetupForm}
          onSubmit={registerWorktree}
        />

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
          <select
            value={createForm.worktreeId}
            onChange={(event) => {
              const worktreeId = event.target.value;
              const anchor = selectedWorktreeAnchor(snapshot, worktreeId);
              setCreateForm({
                ...createForm,
                worktreeId,
                projectId: anchor?.projectId ?? createForm.projectId,
              });
            }}
            disabled={snapshot.worktreeAnchors.length === 0 || busyAction !== null}
            aria-label="Worktree"
          >
            {snapshot.worktreeAnchors.length === 0 ? (
              <option value="">No registered worktrees</option>
            ) : (
              <>
                <option value="">No worktree</option>
                {snapshot.worktreeAnchors.map((anchor) => (
                  <option key={anchor.id} value={anchor.id}>
                    {formatWorktreeAnchor(anchor)}
                  </option>
                ))}
              </>
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

        <div className="task-review-layout">
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
                      const isSelected = detail.taskId === task.id;
                      const isBusy =
                        isRunBusy ||
                        busyAction === `update:${task.id}` ||
                        busyAction === `archive:${task.id}`;

                      return (
                        <section
                          className={`task-card${isSelected ? ' selected' : ''}`}
                          key={task.id}
                        >
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
                                  onClick={() => void loadTaskDetail(task.id)}
                                  disabled={isBusy}
                                  title="Open task detail"
                                  aria-label={`Open detail for ${task.title}`}
                                >
                                  <ScrollText size={16} aria-hidden="true" />
                                </button>
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
          <TaskRunDetailPanel
            detail={detail}
            onClose={() => setDetail({ taskId: null, status: 'idle', snapshot: null, error: null })}
            onReload={() => {
              if (detail.taskId) {
                void loadTaskDetail(detail.taskId);
              }
            }}
          />
        </div>
      </section>
    </main>
  );
}

interface RegisterWorktreeFormProps {
  form: WorktreeSetupForm;
  busy: boolean;
  available: boolean;
  onChange(form: WorktreeSetupForm): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
}

function RegisterWorktreeForm({
  form,
  busy,
  available,
  onChange,
  onSubmit,
}: RegisterWorktreeFormProps) {
  const canSubmit =
    available && !busy && form.projectName.trim().length > 0 && form.worktreePath.trim().length > 0;

  return (
    <form className="setup-panel" onSubmit={onSubmit} aria-label="Register repo worktree">
      <input
        value={form.projectName}
        onChange={(event) => onChange({ ...form, projectName: event.target.value })}
        disabled={!available || busy}
        placeholder="Project name"
        aria-label="Project name"
      />
      <input
        value={form.repoRootPath}
        onChange={(event) => onChange({ ...form, repoRootPath: event.target.value })}
        disabled={!available || busy}
        placeholder="Repo root path"
        aria-label="Repo root path"
      />
      <input
        value={form.worktreePath}
        onChange={(event) => onChange({ ...form, worktreePath: event.target.value })}
        disabled={!available || busy}
        placeholder="Worktree path"
        aria-label="Worktree path"
      />
      <input
        value={form.branchName}
        onChange={(event) => onChange({ ...form, branchName: event.target.value })}
        disabled={!available || busy}
        placeholder="Branch"
        aria-label="Branch name"
      />
      <button className="primary-action" type="submit" disabled={!canSubmit}>
        <GitBranch size={17} aria-hidden="true" />
        Register
      </button>
    </form>
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

interface TaskRunDetailPanelProps {
  detail: DetailState;
  onClose(): void;
  onReload(): void;
}

function TaskRunDetailPanel({ detail, onClose, onReload }: TaskRunDetailPanelProps) {
  const snapshot = detail.snapshot;

  return (
    <aside className="detail-panel" aria-label="Task run detail">
      <header className="detail-header">
        <div>
          <p className="eyebrow">Task detail</p>
          <h2>{snapshot?.task.record.title ?? 'No task open'}</h2>
        </div>
        <div className="detail-actions">
          <button
            className="icon-button"
            type="button"
            onClick={onReload}
            disabled={!detail.taskId || detail.status === 'loading'}
            title="Reload detail"
            aria-label="Reload task detail"
          >
            <RefreshCw size={16} aria-hidden="true" />
          </button>
          <button
            className="icon-button"
            type="button"
            onClick={onClose}
            disabled={!detail.taskId}
            title="Close detail"
            aria-label="Close task detail"
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>
      </header>

      {detail.status === 'idle' && <p className="detail-empty">Select a task to inspect.</p>}

      {detail.status === 'loading' && (
        <p className="detail-state" role="status">
          Loading task detail...
        </p>
      )}

      {detail.status === 'failed' && (
        <section className="notice error detail-notice" role="status">
          <AlertCircle size={18} aria-hidden="true" />
          <span>{detail.error}</span>
        </section>
      )}

      {snapshot && (
        <div className="detail-content">
          <DetailSection title="Anchors" icon={Route}>
            <dl className="anchor-grid">
              <DetailTerm label="Project" value={snapshot.task.project?.name} />
              <DetailTerm label="Repo" value={snapshot.task.repo?.name} />
              <DetailTerm label="Branch" value={snapshot.task.branch?.name} />
              <DetailTerm label="Worktree" value={snapshot.task.worktree?.path} compactPathValue />
              <DetailTerm label="Execution" value={snapshot.task.record.executionState} />
              <DetailTerm label="Attention" value={snapshot.task.record.attentionState} />
            </dl>
          </DetailSection>

          <DetailSection title="Runs" icon={Activity}>
            {snapshot.runs.length === 0 ? (
              <p className="detail-empty">No runs recorded.</p>
            ) : (
              <div className="run-history">
                {snapshot.runs.map((run) => (
                  <RunDetailCard key={run.run.id} run={run} />
                ))}
              </div>
            )}
          </DetailSection>

          <DetailSection title="Task Artifacts" icon={FileText}>
            <ArtifactBucketSummary groups={snapshot.unlinkedArtifacts} />
            {snapshot.unlinkedValidationRuns.length > 0 && (
              <ValidationList validationRuns={snapshot.unlinkedValidationRuns} />
            )}
          </DetailSection>

          <DetailSection title="Timeline" icon={ScrollText}>
            {snapshot.eventTimeline.length === 0 ? (
              <p className="detail-empty">No events recorded.</p>
            ) : (
              <ol className="event-timeline">
                {snapshot.eventTimeline.map((event) => (
                  <EventTimelineItem key={event.id} event={event} />
                ))}
              </ol>
            )}
          </DetailSection>
        </div>
      )}
    </aside>
  );
}

interface DetailSectionProps {
  title: string;
  icon: LucideIcon;
  children: ReactNode;
}

function DetailSection({ title, icon: Icon, children }: DetailSectionProps) {
  return (
    <section className="detail-section">
      <header>
        <Icon size={16} aria-hidden="true" />
        <h3>{title}</h3>
      </header>
      {children}
    </section>
  );
}

interface DetailTermProps {
  label: string;
  value?: string;
  compactPathValue?: boolean;
}

function DetailTerm({ label, value, compactPathValue = false }: DetailTermProps) {
  return (
    <div>
      <dt>{label}</dt>
      <dd title={value}>{value ? (compactPathValue ? compactPath(value) : value) : 'Unlinked'}</dd>
    </div>
  );
}

interface RunDetailCardProps {
  run: TaskRunDetailRun;
}

function RunDetailCard({ run }: RunDetailCardProps) {
  const artifactTotal = countArtifacts(run.artifacts);
  const latestValidation = run.validationRuns[0]?.run;

  return (
    <article className="run-detail-card">
      <header>
        <div>
          <h4>{run.run.id}</h4>
          <p>{formatDateTime(run.run.completedAt ?? run.run.startedAt ?? run.run.createdAt)}</p>
        </div>
        <span className={`state-pill ${run.run.executionState}`}>{run.run.executionState}</span>
      </header>
      <div className="metric-row">
        <span>{artifactTotal} artifacts</span>
        <span>{run.validationRuns.length} validations</span>
        {run.run.exitCode !== undefined && <span>exit {run.run.exitCode}</span>}
        {latestValidation && <span>{latestValidation.status}</span>}
      </div>
      <ArtifactBucketSummary groups={run.artifacts} />
      <ValidationList validationRuns={run.validationRuns} />
      {run.events.length > 0 && (
        <ol className="mini-events">
          {run.events.slice(-3).map((event) => (
            <EventTimelineItem key={event.id} event={event} compact />
          ))}
        </ol>
      )}
    </article>
  );
}

interface ArtifactBucketSummaryProps {
  groups: TaskRunDetailArtifactGroups;
}

function ArtifactBucketSummary({ groups }: ArtifactBucketSummaryProps) {
  const buckets = [
    ['Final', groups.finalResponses],
    ['Raw', groups.rawEventStreams],
    ['Diff', groups.diffs],
    ['Validation', groups.validationLogs],
    ['Notes', groups.notes],
    ['Screens', groups.screenshots],
    ['Handoffs', groups.handoffs],
    ['Summaries', groups.summaries],
    ['Other', groups.other],
  ] as const;
  const visibleBuckets = buckets.filter(([, artifacts]) => artifacts.length > 0);

  if (visibleBuckets.length === 0) {
    return <p className="detail-empty">No artifacts recorded.</p>;
  }

  return (
    <div className="artifact-buckets">
      {visibleBuckets.map(([label, artifacts]) => (
        <div className="artifact-bucket" key={label}>
          <strong>
            {label}
            <span>{artifacts.length}</span>
          </strong>
          <ul>
            {artifacts.slice(0, 2).map((artifact) => (
              <li key={artifact.id}>
                <span title={artifact.title}>{artifact.title}</span>
                <small>{artifactPreview(artifact)}</small>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

interface ValidationListProps {
  validationRuns: TaskRunDetailValidationRun[];
}

function ValidationList({ validationRuns }: ValidationListProps) {
  if (validationRuns.length === 0) {
    return null;
  }

  return (
    <div className="validation-list">
      {validationRuns.map(({ run, outputArtifact }) => (
        <div className="validation-row" key={run.id}>
          <ShieldCheck size={15} aria-hidden="true" />
          <span>{run.command}</span>
          <strong>{run.status}</strong>
          {run.exitCode !== undefined && <small>exit {run.exitCode}</small>}
          {outputArtifact && <small title={outputArtifact.title}>{outputArtifact.title}</small>}
        </div>
      ))}
    </div>
  );
}

interface EventTimelineItemProps {
  event: Event;
  compact?: boolean;
}

function EventTimelineItem({ event, compact = false }: EventTimelineItemProps) {
  return (
    <li>
      <time dateTime={event.occurredAt}>{formatDateTime(event.occurredAt)}</time>
      <span>{event.kind}</span>
      {!compact && <small>{eventSummary(event)}</small>}
    </li>
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

function countArtifacts(groups: TaskRunDetailArtifactGroups): number {
  return Object.values(groups).reduce((total, artifacts) => total + artifacts.length, 0);
}

function artifactPreview(artifact: Artifact): string {
  if (artifact.uri) {
    return compactPath(artifact.uri);
  }

  if (artifact.content) {
    return artifact.content.replace(/\s+/g, ' ').slice(0, 92);
  }

  return formatDateTime(artifact.createdAt);
}

function eventSummary(event: Event): string {
  const entries = Object.entries(event.payload).slice(0, 3);

  if (entries.length === 0) {
    return event.id;
  }

  return entries
    .map(([key, value]) => `${key}: ${String(value)}`)
    .join(' | ')
    .slice(0, 140);
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(value));
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

function nextCreateFormAnchorDefaults(
  current: DraftTaskForm,
  snapshot: TaskDashboardSnapshot,
): Pick<DraftTaskForm, 'projectId' | 'worktreeId'> {
  const currentAnchor = selectedWorktreeAnchor(snapshot, current.worktreeId);
  const fallbackAnchor = currentAnchor ?? snapshot.worktreeAnchors[0];

  return {
    projectId: fallbackAnchor?.projectId ?? (current.projectId || snapshot.projects[0]?.id || ''),
    worktreeId: fallbackAnchor?.id ?? '',
  };
}

function selectedWorktreeAnchor(
  snapshot: TaskDashboardSnapshot,
  worktreeId: EntityId,
): TaskDashboardWorktreeAnchor | undefined {
  return snapshot.worktreeAnchors.find((anchor) => anchor.id === worktreeId);
}

function normalizeWorktreeSetupInput(
  form: WorktreeSetupForm,
): RegisterTaskWorktreeInput | undefined {
  const projectName = form.projectName.trim();
  const worktreePath = form.worktreePath.trim();
  const repoRootPath = form.repoRootPath.trim() || worktreePath;

  if (!projectName || !worktreePath) {
    return undefined;
  }

  return {
    projectName,
    repoRootPath,
    worktreePath,
    ...(form.repoName.trim() ? { repoName: form.repoName.trim() } : {}),
    ...(form.branchName.trim() ? { branchName: form.branchName.trim() } : {}),
  };
}

function formatWorktreeAnchor(anchor: TaskDashboardWorktreeAnchor): string {
  return [anchor.project, anchor.repo, anchor.branch, compactPath(anchor.path)]
    .filter(Boolean)
    .join(' / ');
}
