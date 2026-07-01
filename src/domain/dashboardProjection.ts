import type { DomainRecords, ExecutionState, Task } from './model';

export type DashboardGroupId =
  'needs_action_now' | 'review_decide' | 'working' | 'waiting' | 'later';

export interface DashboardTask {
  id: string;
  title: string;
  summary: string;
  project: string;
  executionState: ExecutionState;
  attentionState: Task['attentionState'];
  repo?: string;
  branch?: string;
  worktreePath?: string;
  updatedAt: string;
}

export interface DashboardGroup {
  id: DashboardGroupId;
  title: string;
  tasks: DashboardTask[];
}

export const dashboardGroupOrder: Array<{ id: DashboardGroupId; title: string }> = [
  { id: 'needs_action_now', title: 'Needs action now' },
  { id: 'review_decide', title: 'Review / decide' },
  { id: 'working', title: 'Working' },
  { id: 'waiting', title: 'Waiting' },
  { id: 'later', title: 'Later' },
];

export function projectOpenTaskDashboard(records: DomainRecords): DashboardGroup[] {
  const projectsById = new Map(records.projects.map((project) => [project.id, project]));
  const reposById = new Map(records.repos.map((repo) => [repo.id, repo]));
  const branchesById = new Map(records.branches.map((branch) => [branch.id, branch]));
  const worktreesById = new Map(records.worktrees.map((worktree) => [worktree.id, worktree]));

  const groups = new Map<DashboardGroupId, DashboardTask[]>(
    dashboardGroupOrder.map((group) => [group.id, []]),
  );

  for (const task of records.tasks) {
    if (isClosedTask(task)) {
      continue;
    }

    const groupId = getDashboardGroupId(task);
    const project = projectsById.get(task.projectId);
    const repo = task.repoId ? reposById.get(task.repoId) : undefined;
    const branch = task.branchId ? branchesById.get(task.branchId) : undefined;
    const worktree = task.worktreeId ? worktreesById.get(task.worktreeId) : undefined;

    groups.get(groupId)?.push({
      id: task.id,
      title: task.title,
      summary: task.summary,
      project: project?.name ?? 'Unassigned project',
      executionState: task.executionState,
      attentionState: task.attentionState,
      repo: repo?.name,
      branch: branch?.name,
      worktreePath: worktree?.path,
      updatedAt: task.updatedAt,
    });
  }

  return dashboardGroupOrder.map((group) => ({
    ...group,
    tasks: sortDashboardTasks(groups.get(group.id) ?? []),
  }));
}

export function getDashboardGroupId(task: Task): DashboardGroupId {
  if (task.attentionState === 'needs_action_now') {
    return 'needs_action_now';
  }

  if (task.attentionState === 'needs_review') {
    return 'review_decide';
  }

  if (task.executionState === 'running' || task.executionState === 'queued') {
    return 'working';
  }

  if (task.attentionState === 'waiting_on_agent' || task.attentionState === 'waiting_on_external') {
    return 'waiting';
  }

  return 'later';
}

function isClosedTask(task: Task): boolean {
  return task.executionState === 'archived' || task.executionState === 'abandoned';
}

function sortDashboardTasks(tasks: DashboardTask[]): DashboardTask[] {
  return [...tasks].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}
