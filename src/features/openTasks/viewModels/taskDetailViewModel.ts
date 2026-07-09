import type {
  TaskRunDetailArtifactGroups,
  TaskRunDetailSnapshot,
  TaskRunDetailValidationRun,
} from '../../../capabilities/taskRunDetail';
import type { Artifact, Event } from '../../../domain/model';
import { compactPath, formatDateTime } from '../../../app/viewModels/formatting';

export type TaskRunDetailPanelStatus = 'idle' | 'loading' | 'loaded' | 'failed';

export interface TaskRunDetailProjectionInput {
  taskId: string | null;
  status: TaskRunDetailPanelStatus;
  snapshot: TaskRunDetailSnapshot | null;
  error: string | null;
}

export interface TaskRunDetailPanelViewModel {
  title: string;
  status: TaskRunDetailPanelStatus;
  error: string | null;
  reloadDisabled: boolean;
  closeDisabled: boolean;
  anchors: DetailTermViewModel[];
  runs: RunDetailCardViewModel[];
  unlinkedArtifacts: ArtifactBucketViewModel[];
  unlinkedValidations: ValidationRowViewModel[];
  timeline: EventTimelineItemViewModel[];
}

export interface DetailTermViewModel {
  label: string;
  value: string;
  title?: string;
}

export interface RunDetailCardViewModel {
  id: string;
  timestampLabel: string;
  executionState: string;
  metrics: string[];
  artifacts: ArtifactBucketViewModel[];
  validations: ValidationRowViewModel[];
  recentEvents: EventTimelineItemViewModel[];
}

export interface ArtifactBucketViewModel {
  label: string;
  count: number;
  artifacts: ArtifactPreviewViewModel[];
}

export interface ArtifactPreviewViewModel {
  id: string;
  title: string;
  preview: string;
}

export interface ValidationRowViewModel {
  id: string;
  command: string;
  status: string;
  exitCodeLabel?: string;
  outputArtifactTitle?: string;
}

export interface EventTimelineItemViewModel {
  id: string;
  occurredAt: string;
  occurredAtLabel: string;
  kind: string;
  summary?: string;
}

export function createTaskRunDetailPanelViewModel({
  taskId,
  status,
  snapshot,
  error,
}: TaskRunDetailProjectionInput): TaskRunDetailPanelViewModel {
  return {
    title: snapshot?.task.record.title ?? 'No task open',
    status,
    error,
    reloadDisabled: !taskId || status === 'loading',
    closeDisabled: !taskId,
    anchors: snapshot ? createDetailAnchors(snapshot) : [],
    runs: snapshot?.runs.map((run) => {
      const artifactTotal = countArtifacts(run.artifacts);
      const latestValidation = run.validationRuns[0]?.run;
      const metrics = [
        `${artifactTotal} artifacts`,
        `${run.validationRuns.length} validations`,
        ...(run.run.exitCode === undefined ? [] : [`exit ${run.run.exitCode}`]),
        ...(latestValidation === undefined ? [] : [latestValidation.status]),
      ];

      return {
        id: run.run.id,
        timestampLabel: formatDateTime(
          run.run.completedAt ?? run.run.startedAt ?? run.run.createdAt,
        ),
        executionState: run.run.executionState,
        metrics,
        artifacts: createArtifactBuckets(run.artifacts),
        validations: run.validationRuns.map(createValidationRow),
        recentEvents: run.events.slice(-3).map((event) => createEventTimelineItem(event, true)),
      };
    }) ?? [],
    unlinkedArtifacts: snapshot ? createArtifactBuckets(snapshot.unlinkedArtifacts) : [],
    unlinkedValidations: snapshot?.unlinkedValidationRuns.map(createValidationRow) ?? [],
    timeline: snapshot?.eventTimeline.map((event) => createEventTimelineItem(event, false)) ?? [],
  };
}

function createDetailAnchors(snapshot: TaskRunDetailSnapshot): DetailTermViewModel[] {
  return [
    createDetailTerm('Project', snapshot.task.project?.name),
    createDetailTerm('Repo', snapshot.task.repo?.name),
    createDetailTerm('Branch', snapshot.task.branch?.name),
    createDetailTerm('Worktree', snapshot.task.worktree?.path, true),
    createDetailTerm('Execution', snapshot.task.record.executionState),
    createDetailTerm('Attention', snapshot.task.record.attentionState),
  ];
}

function createDetailTerm(
  label: string,
  rawValue: string | undefined,
  compactPathValue = false,
): DetailTermViewModel {
  if (!rawValue) {
    return { label, value: 'Unlinked' };
  }

  return {
    label,
    value: compactPathValue ? compactPath(rawValue) : rawValue,
    title: rawValue,
  };
}

export function countArtifacts(groups: TaskRunDetailArtifactGroups): number {
  return Object.values(groups).reduce((total, artifacts) => total + artifacts.length, 0);
}

function createArtifactBuckets(groups: TaskRunDetailArtifactGroups): ArtifactBucketViewModel[] {
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

  return buckets
    .filter(([, artifacts]) => artifacts.length > 0)
    .map(([label, artifacts]) => ({
      label,
      count: artifacts.length,
      artifacts: artifacts.slice(0, 2).map((artifact) => ({
        id: artifact.id,
        title: artifact.title,
        preview: artifactPreview(artifact),
      })),
    }));
}

export function artifactPreview(artifact: Artifact): string {
  if (artifact.uri) {
    return compactPath(artifact.uri);
  }

  if (artifact.content) {
    return artifact.content.replace(/\s+/g, ' ').slice(0, 92);
  }

  return formatDateTime(artifact.createdAt);
}

function createValidationRow({
  run,
  outputArtifact,
}: TaskRunDetailValidationRun): ValidationRowViewModel {
  return {
    id: run.id,
    command: run.command,
    status: run.status,
    ...(run.exitCode === undefined ? {} : { exitCodeLabel: `exit ${run.exitCode}` }),
    ...(outputArtifact === undefined ? {} : { outputArtifactTitle: outputArtifact.title }),
  };
}

function createEventTimelineItem(event: Event, compact: boolean): EventTimelineItemViewModel {
  return {
    id: event.id,
    occurredAt: event.occurredAt,
    occurredAtLabel: formatDateTime(event.occurredAt),
    kind: event.kind,
    ...(compact ? {} : { summary: eventSummary(event) }),
  };
}

export function eventSummary(event: Event): string {
  const entries = Object.entries(event.payload).slice(0, 3);

  if (entries.length === 0) {
    return event.id;
  }

  return entries
    .map(([key, value]) => `${key}: ${String(value)}`)
    .join(' | ')
    .slice(0, 140);
}
