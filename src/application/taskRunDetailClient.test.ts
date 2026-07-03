import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryEventStore } from '../domain/eventStore';
import {
  type Artifact,
  type Branch,
  type DomainRecords,
  type Event,
  type IsoDateTime,
  type Project,
  type Repo,
  type Task,
  type TaskRun,
  type ValidationRun,
  type Worktree,
} from '../domain/model';
import { InMemoryOpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import { InMemoryTaskRunStore } from '../domain/taskRunStore';
import { InMemoryValidationRunStore } from '../domain/validationRunStore';
import {
  createStoreBackedTaskRunDetailClient,
  TaskRunDetailTaskNotFoundError,
} from './taskRunDetailClient';

const taskId = 'task-detail';
const projectId = 'project-orchestrator';
const repoId = 'repo-orchestrator';
const branchId = 'branch-worker';
const worktreeId = 'worktree-worker';

describe('task run detail client', () => {
  it('loads task anchors and run history ordered newest first for review', async () => {
    const client = createFixture({
      taskRuns: [
        baseTaskRun({
          id: 'run-old',
          startedAt: '2026-07-02T08:00:00.000Z',
          completedAt: '2026-07-02T08:30:00.000Z',
          createdAt: '2026-07-02T08:00:00.000Z',
        }),
        baseTaskRun({
          id: 'run-new',
          startedAt: '2026-07-02T09:00:00.000Z',
          completedAt: '2026-07-02T09:15:00.000Z',
          createdAt: '2026-07-02T09:00:00.000Z',
        }),
      ],
    }).client;

    const detail = await client.loadTaskRunDetail(taskId);

    expect(detail.task).toMatchObject({
      record: expect.objectContaining({ id: taskId, title: 'Review task detail' }),
      project: expect.objectContaining({ id: projectId, name: 'Codex Orchestrator' }),
      repo: expect.objectContaining({ id: repoId, rootPath: 'C:/Repos/Codex Orchestrator' }),
      branch: expect.objectContaining({ id: branchId, name: 'worker/detail' }),
      worktree: expect.objectContaining({ id: worktreeId, path: 'C:/Worktrees/044' }),
    });
    expect(detail.runs.map((run) => run.run.id)).toEqual(['run-new', 'run-old']);
  });

  it('throws a focused missing task error before querying run-scoped stores', async () => {
    const fixture = createFixture({
      records: recordsWith({ tasks: [] }),
    });

    await expect(fixture.client.loadTaskRunDetail('task-missing')).rejects.toThrow(
      TaskRunDetailTaskNotFoundError,
    );
  });

  it('classifies artifacts by kind for each run and preserves unlinked task-level artifacts', async () => {
    const fixture = createFixture({
      taskRuns: [baseTaskRun({ id: 'run-1' })],
      artifacts: [
        baseArtifact({ id: 'artifact-final', taskRunId: 'run-1', kind: 'final_response' }),
        baseArtifact({ id: 'artifact-raw', taskRunId: 'run-1', kind: 'raw_event_stream' }),
        baseArtifact({ id: 'artifact-diff', taskRunId: 'run-1', kind: 'diff' }),
        baseArtifact({ id: 'artifact-validation', taskRunId: 'run-1', kind: 'validation_log' }),
        baseArtifact({ id: 'artifact-note', taskRunId: undefined, kind: 'note' }),
        baseArtifact({ id: 'artifact-summary', taskRunId: undefined, kind: 'summary' }),
      ],
    });

    const detail = await fixture.client.loadTaskRunDetail(taskId);
    const run = detail.runs[0];

    expect(run?.artifacts.finalResponses.map((artifact) => artifact.id)).toEqual([
      'artifact-final',
    ]);
    expect(run?.artifacts.rawEventStreams.map((artifact) => artifact.id)).toEqual(['artifact-raw']);
    expect(run?.artifacts.diffs.map((artifact) => artifact.id)).toEqual(['artifact-diff']);
    expect(run?.artifacts.validationLogs.map((artifact) => artifact.id)).toEqual([
      'artifact-validation',
    ]);
    expect(detail.unlinkedArtifacts.notes.map((artifact) => artifact.id)).toEqual([
      'artifact-note',
    ]);
    expect(detail.unlinkedArtifacts.summaries.map((artifact) => artifact.id)).toEqual([
      'artifact-summary',
    ]);
  });

  it('links validation runs directly by task run and indirectly by output artifact', async () => {
    const fixture = createFixture({
      taskRuns: [baseTaskRun({ id: 'run-1' }), baseTaskRun({ id: 'run-2' })],
      artifacts: [
        baseArtifact({
          id: 'artifact-linked-log',
          taskRunId: 'run-2',
          kind: 'validation_log',
        }),
        baseArtifact({
          id: 'artifact-unlinked-log',
          taskRunId: undefined,
          kind: 'validation_log',
        }),
      ],
      validationRuns: [
        baseValidationRun({
          id: 'validation-direct',
          taskRunId: 'run-1',
          outputArtifactId: undefined,
        }),
        baseValidationRun({
          id: 'validation-indirect',
          taskRunId: undefined,
          outputArtifactId: 'artifact-linked-log',
        }),
        baseValidationRun({
          id: 'validation-unlinked',
          taskRunId: undefined,
          outputArtifactId: 'artifact-unlinked-log',
        }),
      ],
    });

    const detail = await fixture.client.loadTaskRunDetail(taskId);
    const run1 = detail.runs.find((run) => run.run.id === 'run-1');
    const run2 = detail.runs.find((run) => run.run.id === 'run-2');

    expect(run1?.validationRuns.map((validation) => validation.run.id)).toEqual([
      'validation-direct',
    ]);
    expect(run2?.validationRuns.map((validation) => validation.run.id)).toEqual([
      'validation-indirect',
    ]);
    expect(run2?.validationRuns[0]?.outputArtifact?.id).toBe('artifact-linked-log');
    expect(detail.unlinkedValidationRuns.map((validation) => validation.run.id)).toEqual([
      'validation-unlinked',
    ]);
    expect(detail.unlinkedValidationRuns[0]?.outputArtifact?.id).toBe('artifact-unlinked-log');
    expect(detail.unlinkedArtifacts.validationLogs).toEqual([]);
  });

  it('returns a chronological task event timeline plus run-local events', async () => {
    const fixture = createFixture({
      taskRuns: [baseTaskRun({ id: 'run-1' })],
      events: [
        baseEvent({
          id: 'event-3',
          kind: 'run_completed',
          taskRunId: 'run-1',
          occurredAt: '2026-07-02T10:03:00.000Z',
        }),
        baseEvent({
          id: 'event-1',
          kind: 'task_updated',
          taskRunId: undefined,
          occurredAt: '2026-07-02T10:01:00.000Z',
        }),
        baseEvent({
          id: 'event-2',
          kind: 'run_started',
          taskRunId: 'run-1',
          occurredAt: '2026-07-02T10:02:00.000Z',
        }),
      ],
    });

    const detail = await fixture.client.loadTaskRunDetail(taskId);

    expect(detail.eventTimeline.map((event) => event.id)).toEqual([
      'event-1',
      'event-2',
      'event-3',
    ]);
    expect(detail.runs[0]?.events.map((event) => event.id)).toEqual(['event-2', 'event-3']);
  });
});

interface FixtureInput {
  records?: DomainRecords;
  taskRuns?: TaskRun[];
  artifacts?: Artifact[];
  validationRuns?: ValidationRun[];
  events?: Event[];
}

function createFixture(input: FixtureInput = {}) {
  const records = input.records ?? baseRecords();

  return {
    client: createStoreBackedTaskRunDetailClient({
      dashboard: new InMemoryOpenTaskDashboardStore(records),
      taskRun: new InMemoryTaskRunStore(
        sequenceIds('unused-run'),
        fixedClock('2026-07-02T12:00:00.000Z'),
        input.taskRuns ?? [],
      ),
      artifact: new InMemoryArtifactStore(
        sequenceIds('unused-artifact'),
        fixedClock('2026-07-02T12:00:00.000Z'),
        input.artifacts ?? [],
      ),
      event: new InMemoryEventStore(
        sequenceIds('unused-event'),
        fixedClock('2026-07-02T12:00:00.000Z'),
        input.events ?? [],
      ),
      validationRun: new InMemoryValidationRunStore(
        sequenceIds('unused-validation'),
        fixedClock('2026-07-02T12:00:00.000Z'),
        input.validationRuns ?? [],
      ),
    }),
  };
}

function baseRecords(): DomainRecords {
  return recordsWith({});
}

function recordsWith(overrides: Partial<DomainRecords>): DomainRecords {
  return {
    projects: [baseProject()],
    repos: [baseRepo()],
    branches: [baseBranch()],
    worktrees: [baseWorktree()],
    conversations: [],
    tasks: [baseTask()],
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
    ...overrides,
  };
}

function baseProject(overrides: Partial<Project> = {}): Project {
  return {
    id: projectId,
    name: 'Codex Orchestrator',
    createdAt: '2026-07-02T07:00:00.000Z',
    updatedAt: '2026-07-02T07:00:00.000Z',
    ...overrides,
  };
}

function baseRepo(overrides: Partial<Repo> = {}): Repo {
  return {
    id: repoId,
    projectId,
    name: 'Codex Orchestrator',
    rootPath: 'C:/Repos/Codex Orchestrator',
    defaultBranch: 'main',
    createdAt: '2026-07-02T07:01:00.000Z',
    updatedAt: '2026-07-02T07:01:00.000Z',
    ...overrides,
  };
}

function baseBranch(overrides: Partial<Branch> = {}): Branch {
  return {
    id: branchId,
    repoId,
    name: 'worker/detail',
    baseBranch: 'main',
    createdAt: '2026-07-02T07:02:00.000Z',
    updatedAt: '2026-07-02T07:02:00.000Z',
    ...overrides,
  };
}

function baseWorktree(overrides: Partial<Worktree> = {}): Worktree {
  return {
    id: worktreeId,
    repoId,
    branchId,
    path: 'C:/Worktrees/044',
    isMain: false,
    isDirty: false,
    createdAt: '2026-07-02T07:03:00.000Z',
    updatedAt: '2026-07-02T07:03:00.000Z',
    ...overrides,
  };
}

function baseTask(overrides: Partial<Task> = {}): Task {
  return {
    id: taskId,
    projectId,
    repoId,
    branchId,
    worktreeId,
    conversationIds: [],
    title: 'Review task detail',
    summary: 'Load run detail data for the review UI.',
    executionState: 'completed',
    attentionState: 'needs_review',
    priority: 'normal',
    createdAt: '2026-07-02T07:04:00.000Z',
    updatedAt: '2026-07-02T07:04:00.000Z',
    ...overrides,
  };
}

function baseTaskRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: 'run-1',
    taskId,
    worktreeId,
    executionState: 'completed',
    startedAt: '2026-07-02T08:00:00.000Z',
    completedAt: '2026-07-02T08:30:00.000Z',
    createdAt: '2026-07-02T08:00:00.000Z',
    updatedAt: '2026-07-02T08:30:00.000Z',
    ...overrides,
  };
}

function baseArtifact(overrides: Partial<Artifact> = {}): Artifact {
  const artifact: Artifact = {
    id: 'artifact-1',
    taskId,
    taskRunId: 'run-1',
    kind: 'final_response',
    title: 'Artifact',
    content: 'Artifact content',
    createdAt: '2026-07-02T09:00:00.000Z',
  };

  return applyOptionalArtifactOverrides(artifact, overrides);
}

function applyOptionalArtifactOverrides(
  artifact: Artifact,
  overrides: Partial<Artifact>,
): Artifact {
  const next = { ...artifact, ...overrides };

  if ('taskRunId' in overrides && overrides.taskRunId === undefined) {
    delete next.taskRunId;
  }

  return next;
}

function baseValidationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  const validationRun: ValidationRun = {
    id: 'validation-1',
    taskId,
    taskRunId: 'run-1',
    command: 'npm run test',
    status: 'passed',
    startedAt: '2026-07-02T09:30:00.000Z',
    completedAt: '2026-07-02T09:31:00.000Z',
    exitCode: 0,
    outputArtifactId: 'artifact-validation',
    createdAt: '2026-07-02T09:30:00.000Z',
    updatedAt: '2026-07-02T09:31:00.000Z',
  };

  return applyOptionalValidationOverrides(validationRun, overrides);
}

function applyOptionalValidationOverrides(
  validationRun: ValidationRun,
  overrides: Partial<ValidationRun>,
): ValidationRun {
  const next = { ...validationRun, ...overrides };

  if ('taskRunId' in overrides && overrides.taskRunId === undefined) {
    delete next.taskRunId;
  }

  if ('outputArtifactId' in overrides && overrides.outputArtifactId === undefined) {
    delete next.outputArtifactId;
  }

  return next;
}

function baseEvent(overrides: Partial<Event> = {}): Event {
  const event: Event = {
    id: 'event-1',
    kind: 'run_started',
    occurredAt: '2026-07-02T10:00:00.000Z',
    projectId,
    taskId,
    taskRunId: 'run-1',
    payload: {},
  };

  return applyOptionalEventOverrides(event, overrides);
}

function applyOptionalEventOverrides(event: Event, overrides: Partial<Event>): Event {
  const next = { ...event, ...overrides };

  if ('taskRunId' in overrides && overrides.taskRunId === undefined) {
    delete next.taskRunId;
  }

  return next;
}

function fixedClock(now: IsoDateTime): { now(): IsoDateTime } {
  return {
    now: () => now,
  };
}

function sequenceIds(prefix: string): { nextId(): string } {
  let next = 1;

  return {
    nextId: () => `${prefix}-${next++}`,
  };
}
