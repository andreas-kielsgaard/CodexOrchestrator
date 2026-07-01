export type DashboardGroupId =
  'needs_action_now' | 'review_decide' | 'working' | 'waiting' | 'later';

export type ExecutionState = 'draft' | 'queued' | 'running' | 'blocked' | 'completed';

export interface DashboardTask {
  id: string;
  title: string;
  summary: string;
  project: string;
  executionState: ExecutionState;
}

export interface DashboardGroup {
  id: DashboardGroupId;
  title: string;
  tasks: DashboardTask[];
}

export const dashboardGroups: DashboardGroup[] = [
  {
    id: 'needs_action_now',
    title: 'Needs action now',
    tasks: [
      {
        id: 'task-bootstrap-review',
        title: 'Resolve bootstrap blockers',
        summary: 'Install Rust locally before desktop shell verification can run.',
        project: 'Codex Orchestrator',
        executionState: 'blocked',
      },
    ],
  },
  {
    id: 'review_decide',
    title: 'Review / decide',
    tasks: [
      {
        id: 'task-adapter-choice',
        title: 'Choose first Codex adapter',
        summary: 'Compare JSONL exec flow against richer app-server control.',
        project: 'Runtime architecture',
        executionState: 'draft',
      },
    ],
  },
  {
    id: 'working',
    title: 'Working',
    tasks: [
      {
        id: 'task-dashboard-shell',
        title: 'Dashboard shell',
        summary: 'Establish the attention-first task surface with placeholder data.',
        project: 'Codex Orchestrator',
        executionState: 'running',
      },
    ],
  },
  {
    id: 'waiting',
    title: 'Waiting',
    tasks: [
      {
        id: 'task-domain-schema',
        title: 'Domain schema slice',
        summary: 'Waiting for bootstrap review before adding SQLite migrations.',
        project: 'Persistence',
        executionState: 'queued',
      },
    ],
  },
  {
    id: 'later',
    title: 'Later',
    tasks: [
      {
        id: 'task-chatgpt-import',
        title: 'ChatGPT export import',
        summary: 'Archive/search/linking workflow belongs after core task runs.',
        project: 'Future integrations',
        executionState: 'draft',
      },
    ],
  },
];
