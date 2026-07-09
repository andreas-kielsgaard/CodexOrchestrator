import type { ArtifactStore } from '../../domain/artifactStore';
import type { EventStore } from '../../domain/eventStore';
import type {
  Artifact,
  ArtifactKind,
  Branch,
  EntityId,
  Event,
  Project,
  Repo,
  Task,
  TaskRun,
  ValidationRun,
  Worktree,
} from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import type { TaskRunStore } from '../../domain/taskRunStore';
import type { ValidationRunStore } from '../../domain/validationRunStore';

export interface TaskRunDetailClient {
  loadTaskRunDetail(taskId: EntityId): Promise<TaskRunDetailSnapshot>;
}

export interface StoreBackedTaskRunDetailClientStores {
  dashboard: OpenTaskDashboardStore;
  taskRun: TaskRunStore;
  artifact: ArtifactStore;
  event: EventStore;
  validationRun: ValidationRunStore;
}

export interface TaskRunDetailSnapshot {
  task: TaskRunDetailTaskAnchor;
  runs: TaskRunDetailRun[];
  unlinkedArtifacts: TaskRunDetailArtifactGroups;
  unlinkedValidationRuns: TaskRunDetailValidationRun[];
  eventTimeline: Event[];
}

export interface TaskRunDetailTaskAnchor {
  record: Task;
  project?: Project;
  repo?: Repo;
  branch?: Branch;
  worktree?: Worktree;
}

export interface TaskRunDetailRun {
  run: TaskRun;
  artifacts: TaskRunDetailArtifactGroups;
  validationRuns: TaskRunDetailValidationRun[];
  events: Event[];
}

export interface TaskRunDetailValidationRun {
  run: ValidationRun;
  outputArtifact?: Artifact;
}

export interface TaskRunDetailArtifactGroups {
  finalResponses: Artifact[];
  rawEventStreams: Artifact[];
  diffs: Artifact[];
  validationLogs: Artifact[];
  notes: Artifact[];
  screenshots: Artifact[];
  handoffs: Artifact[];
  summaries: Artifact[];
  other: Artifact[];
}

export class TaskRunDetailTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Task not found: ${taskId}`);
    this.name = 'TaskRunDetailTaskNotFoundError';
  }
}

export function createStoreBackedTaskRunDetailClient(
  stores: StoreBackedTaskRunDetailClientStores,
): TaskRunDetailClient {
  return {
    async loadTaskRunDetail(taskId: EntityId): Promise<TaskRunDetailSnapshot> {
      return loadTaskRunDetail(stores, taskId);
    },
  };
}

export async function loadTaskRunDetail(
  stores: StoreBackedTaskRunDetailClientStores,
  taskId: EntityId,
): Promise<TaskRunDetailSnapshot> {
  const records = await stores.dashboard.loadOpenTaskDashboardRecords();
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new TaskRunDetailTaskNotFoundError(taskId);
  }

  const [taskRuns, artifacts, events, validationRuns] = await Promise.all([
    stores.taskRun.queryTaskRuns({ taskId }),
    stores.artifact.queryArtifacts({ taskId }),
    stores.event.queryEvents({ taskId }),
    stores.validationRun.queryValidationRuns({ taskId }),
  ]);
  const runIds = new Set(taskRuns.map((run) => run.id));
  const outputArtifactIdsByValidationRun = new Map(
    validationRuns.flatMap((run) =>
      run.outputArtifactId === undefined ? [] : [[run.id, run.outputArtifactId] as const],
    ),
  );

  return {
    task: {
      record: cloneTask(task),
      project: cloneOptional(records.projects.find((project) => project.id === task.projectId)),
      repo: cloneOptional(records.repos.find((repo) => repo.id === task.repoId)),
      branch: cloneOptional(records.branches.find((branch) => branch.id === task.branchId)),
      worktree: cloneOptional(
        records.worktrees.find((worktree) => worktree.id === task.worktreeId),
      ),
    },
    runs: [...taskRuns].sort(compareRunsForReview).map((run) => {
      const runArtifacts = artifacts.filter((artifact) => artifact.taskRunId === run.id);
      const runValidationRuns = validationRuns.filter((validationRun) =>
        validationRunBelongsToRun(validationRun, run.id, artifacts),
      );
      const runValidationArtifactIds = new Set(
        runValidationRuns.flatMap((validationRun) =>
          validationRun.outputArtifactId === undefined ? [] : [validationRun.outputArtifactId],
        ),
      );

      return {
        run: cloneTaskRun(run),
        artifacts: groupArtifacts([
          ...runArtifacts,
          ...artifacts.filter(
            (artifact) =>
              artifact.taskRunId === undefined &&
              runValidationArtifactIds.has(artifact.id) &&
              !runArtifacts.some((runArtifact) => runArtifact.id === artifact.id),
          ),
        ]),
        validationRuns: runValidationRuns.map((validationRun) =>
          detailValidationRun(validationRun, artifacts),
        ),
        events: events
          .filter((event) => event.taskRunId === run.id)
          .sort(compareEventsChronologically)
          .map(cloneEvent),
      };
    }),
    unlinkedArtifacts: groupArtifacts(
      artifacts.filter(
        (artifact) =>
          artifact.taskRunId === undefined &&
          ![...outputArtifactIdsByValidationRun.values()].includes(artifact.id),
      ),
    ),
    unlinkedValidationRuns: validationRuns
      .filter((validationRun) => !validationRunBelongsToAnyRun(validationRun, runIds, artifacts))
      .sort(compareValidationRunsForReview)
      .map((validationRun) => detailValidationRun(validationRun, artifacts)),
    eventTimeline: events.sort(compareEventsChronologically).map(cloneEvent),
  };
}

function validationRunBelongsToRun(
  validationRun: ValidationRun,
  taskRunId: EntityId,
  artifacts: readonly Artifact[],
): boolean {
  if (validationRun.taskRunId === taskRunId) {
    return true;
  }

  if (validationRun.outputArtifactId === undefined) {
    return false;
  }

  return artifacts.some(
    (artifact) =>
      artifact.id === validationRun.outputArtifactId && artifact.taskRunId === taskRunId,
  );
}

function validationRunBelongsToAnyRun(
  validationRun: ValidationRun,
  runIds: ReadonlySet<EntityId>,
  artifacts: readonly Artifact[],
): boolean {
  if (validationRun.taskRunId !== undefined && runIds.has(validationRun.taskRunId)) {
    return true;
  }

  if (validationRun.outputArtifactId === undefined) {
    return false;
  }

  return artifacts.some(
    (artifact) =>
      artifact.id === validationRun.outputArtifactId &&
      artifact.taskRunId !== undefined &&
      runIds.has(artifact.taskRunId),
  );
}

function detailValidationRun(
  validationRun: ValidationRun,
  artifacts: readonly Artifact[],
): TaskRunDetailValidationRun {
  return {
    run: cloneValidationRun(validationRun),
    outputArtifact: cloneOptional(
      artifacts.find((artifact) => artifact.id === validationRun.outputArtifactId),
    ),
  };
}

function groupArtifacts(artifacts: readonly Artifact[]): TaskRunDetailArtifactGroups {
  const grouped = emptyArtifactGroups();

  for (const artifact of [...artifacts].sort(compareArtifactsChronologically)) {
    groupForKind(grouped, artifact.kind).push(cloneArtifact(artifact));
  }

  return grouped;
}

function emptyArtifactGroups(): TaskRunDetailArtifactGroups {
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

function groupForKind(groups: TaskRunDetailArtifactGroups, kind: ArtifactKind): Artifact[] {
  switch (kind) {
    case 'final_response':
      return groups.finalResponses;
    case 'raw_event_stream':
      return groups.rawEventStreams;
    case 'diff':
      return groups.diffs;
    case 'validation_log':
      return groups.validationLogs;
    case 'note':
      return groups.notes;
    case 'screenshot':
      return groups.screenshots;
    case 'handoff':
      return groups.handoffs;
    case 'summary':
      return groups.summaries;
  }
}

function compareRunsForReview(left: TaskRun, right: TaskRun): number {
  const rightTime = reviewTime(right);
  const leftTime = reviewTime(left);
  const timeComparison = rightTime.localeCompare(leftTime);

  if (timeComparison !== 0) {
    return timeComparison;
  }

  return right.id.localeCompare(left.id);
}

function reviewTime(run: TaskRun): string {
  return run.completedAt ?? run.startedAt ?? run.createdAt;
}

function compareArtifactsChronologically(left: Artifact, right: Artifact): number {
  const createdAtComparison = left.createdAt.localeCompare(right.createdAt);

  if (createdAtComparison !== 0) {
    return createdAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function compareEventsChronologically(left: Event, right: Event): number {
  const occurredAtComparison = left.occurredAt.localeCompare(right.occurredAt);

  if (occurredAtComparison !== 0) {
    return occurredAtComparison;
  }

  return left.id.localeCompare(right.id);
}

function compareValidationRunsForReview(left: ValidationRun, right: ValidationRun): number {
  const rightTime = right.completedAt ?? right.startedAt ?? right.createdAt;
  const leftTime = left.completedAt ?? left.startedAt ?? left.createdAt;
  const timeComparison = rightTime.localeCompare(leftTime);

  if (timeComparison !== 0) {
    return timeComparison;
  }

  return right.id.localeCompare(left.id);
}

function cloneTask(task: Task): Task {
  return { ...task, conversationIds: [...task.conversationIds] };
}

function cloneTaskRun(taskRun: TaskRun): TaskRun {
  return { ...taskRun };
}

function cloneArtifact(artifact: Artifact): Artifact {
  return { ...artifact };
}

function cloneValidationRun(validationRun: ValidationRun): ValidationRun {
  return { ...validationRun };
}

function cloneEvent(event: Event): Event {
  return {
    ...event,
    payload: JSON.parse(JSON.stringify(event.payload)) as Record<string, unknown>,
  };
}

function cloneOptional<T extends object>(value: T | undefined): T | undefined {
  return value === undefined ? undefined : { ...value };
}