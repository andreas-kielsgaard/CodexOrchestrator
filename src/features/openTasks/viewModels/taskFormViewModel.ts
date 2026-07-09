import type {
  TaskDashboardSnapshot,
  TaskDashboardProject,
  TaskDashboardWorktreeAnchor,
} from '../../../capabilities/openTaskDashboard';
import type { EntityId } from '../../../domain/model';
import { compactPath } from '../../../app/viewModels/formatting';
import type {
  AttentionOptionValue,
  ExecutionOptionValue,
  PriorityOptionValue,
} from './taskOptions';

export interface TaskAnchorDraft {
  projectId: EntityId;
  worktreeId: EntityId;
}

export interface TaskComposerFormViewModel extends TaskAnchorDraft {
  title: string;
  summary: string;
  attentionState: AttentionOptionValue;
  executionState: ExecutionOptionValue;
  priority: PriorityOptionValue;
}

export interface TaskComposerProjectOption {
  id: EntityId;
  label: string;
}

export interface TaskComposerWorktreeOption {
  id: EntityId;
  label: string;
}

export function createTaskComposerProjectOptions(
  projects: TaskDashboardProject[],
): TaskComposerProjectOption[] {
  return projects.map((project) => ({
    id: project.id,
    label: project.name,
  }));
}

export function createTaskComposerWorktreeOptions(
  worktreeAnchors: TaskDashboardWorktreeAnchor[],
): TaskComposerWorktreeOption[] {
  return worktreeAnchors.map((anchor) => ({
    id: anchor.id,
    label: formatWorktreeAnchor(anchor),
  }));
}

export function nextCreateFormAnchorDefaults<TDraft extends TaskAnchorDraft>(
  current: TDraft,
  snapshot: TaskDashboardSnapshot,
): Pick<TDraft, 'projectId' | 'worktreeId'> {
  const currentAnchor = selectedWorktreeAnchor(snapshot, current.worktreeId);
  const fallbackAnchor = currentAnchor ?? snapshot.worktreeAnchors[0];

  return {
    projectId: fallbackAnchor?.projectId ?? (current.projectId || snapshot.projects[0]?.id || ''),
    worktreeId: fallbackAnchor?.id ?? '',
  };
}

export function selectedWorktreeAnchor(
  snapshot: TaskDashboardSnapshot,
  worktreeId: EntityId,
): TaskDashboardWorktreeAnchor | undefined {
  return snapshot.worktreeAnchors.find((anchor) => anchor.id === worktreeId);
}

export function formatWorktreeAnchor(anchor: TaskDashboardWorktreeAnchor): string {
  return [anchor.project, anchor.repo, anchor.branch, compactPath(anchor.path)]
    .filter(Boolean)
    .join(' / ');
}
