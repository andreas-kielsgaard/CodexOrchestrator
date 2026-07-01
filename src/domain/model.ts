export type EntityId = string;
export type IsoDateTime = string;

export type ExecutionState =
  'draft' | 'queued' | 'running' | 'blocked' | 'completed' | 'failed' | 'abandoned' | 'archived';

export type AttentionState =
  | 'needs_action_now'
  | 'needs_review'
  | 'waiting_on_agent'
  | 'waiting_on_external'
  | 'consider_later'
  | 'snoozed'
  | 'reference_only';

export type ArtifactKind =
  | 'final_response'
  | 'diff'
  | 'validation_log'
  | 'note'
  | 'screenshot'
  | 'handoff'
  | 'summary'
  | 'raw_event_stream';

export type ValidationStatus = 'queued' | 'running' | 'passed' | 'failed' | 'canceled';

export type EventKind =
  | 'task_created'
  | 'task_updated'
  | 'attention_changed'
  | 'execution_changed'
  | 'run_started'
  | 'run_event'
  | 'run_completed'
  | 'artifact_created'
  | 'validation_started'
  | 'validation_completed';

export interface Project {
  id: EntityId;
  name: string;
  description?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Repo {
  id: EntityId;
  projectId: EntityId;
  name: string;
  rootPath: string;
  defaultBranch: string;
  remoteUrl?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Branch {
  id: EntityId;
  repoId: EntityId;
  name: string;
  baseBranch?: string;
  headSha?: string;
  intent?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Worktree {
  id: EntityId;
  repoId: EntityId;
  branchId?: EntityId;
  path: string;
  isMain: boolean;
  isDirty: boolean;
  lockReason?: string;
  lastScannedAt?: IsoDateTime;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Conversation {
  id: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  provider: 'codex' | 'chatgpt_export' | 'manual';
  externalThreadId?: string;
  title: string;
  summary?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Task {
  id: EntityId;
  projectId: EntityId;
  repoId?: EntityId;
  branchId?: EntityId;
  worktreeId?: EntityId;
  conversationIds: EntityId[];
  title: string;
  summary: string;
  executionState: ExecutionState;
  attentionState: AttentionState;
  priority: 'low' | 'normal' | 'high';
  dueAt?: IsoDateTime;
  snoozedUntil?: IsoDateTime;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface TaskRun {
  id: EntityId;
  taskId: EntityId;
  conversationId?: EntityId;
  worktreeId?: EntityId;
  executionState: ExecutionState;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Artifact {
  id: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  kind: ArtifactKind;
  title: string;
  uri?: string;
  content?: string;
  createdAt: IsoDateTime;
}

export interface ValidationRun {
  id: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  command: string;
  status: ValidationStatus;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  outputArtifactId?: EntityId;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface Event {
  id: EntityId;
  kind: EventKind;
  occurredAt: IsoDateTime;
  projectId?: EntityId;
  taskId?: EntityId;
  taskRunId?: EntityId;
  conversationId?: EntityId;
  artifactId?: EntityId;
  validationRunId?: EntityId;
  payload: Record<string, unknown>;
}

export interface DomainRecords {
  projects: Project[];
  repos: Repo[];
  branches: Branch[];
  worktrees: Worktree[];
  conversations: Conversation[];
  tasks: Task[];
  taskRuns: TaskRun[];
  artifacts: Artifact[];
  validationRuns: ValidationRun[];
  events: Event[];
}
