import type { NavigationOrder, NavigationOrderScope } from './navigationOrder';
import type { AgentSessionSummaryDto } from './contracts';
export type SessionFolderTarget =
  | { readonly kind: 'repository'; readonly repositoryId: string }
  | { readonly kind: 'workflow_instance'; readonly instanceId: string };
export type SessionPlacement = SessionFolderTarget | { readonly kind: 'default' | 'unfiled' };
export interface SessionOrganization {
  readonly sessionId: string;
  readonly placement: SessionPlacement;
  readonly pinnedAt: string | null;
}
export interface NavigationRepository {
  readonly id: string;
  readonly name: string;
}
export interface NavigationInstance {
  readonly id: string;
  readonly name: string;
  readonly repositoryId: string;
  readonly nodes: readonly { readonly id: string; readonly name: string }[];
}
export interface WorkflowSessionOwner {
  readonly sessionId: string;
  readonly instanceId: string;
  readonly nodeId: string;
  readonly nodeName: string;
}
export interface SessionNavigationData {
  readonly branches?: readonly NavigationBranch[];
  readonly sessionBranches?: readonly { readonly sessionId: string; readonly branchId: string }[];
  readonly summaries: readonly AgentSessionSummaryDto[];
  readonly repositories: readonly NavigationRepository[];
  readonly instances: readonly NavigationInstance[];
  readonly owners: readonly WorkflowSessionOwner[];
  readonly organization: readonly SessionOrganization[];
  readonly orders: readonly NavigationOrder[];
}
export interface NavigationBranch {
  readonly worktrees?: readonly NavigationWorktree[];
  readonly id: string;
  readonly repositoryId: string;
  readonly branchRef: string;
  readonly createdAt: string | null;
  readonly paths: readonly string[];
}
export interface NavigationWorktree {
  readonly deviceId: string;
  readonly deviceName: string;
  readonly path: string;
  readonly head: string | null;
  readonly createdAt: string | null;
  readonly dirty: boolean | null;
}
export interface SessionNavigationClient {
  createWorktree?(repositoryId: string, branchRef: string): Promise<unknown>;
  markAttention?(sessionId: string, unread: boolean): Promise<void>;
  previewGitAction?(input: SessionGitActionInput): Promise<SessionGitPreview>;
  executeGitAction?(input: SessionGitActionInput, expected: SessionGitPreview): Promise<string>;
  inspectWorktree?(
    repositoryId: string,
    branchRef: string,
    worktree: NavigationWorktree,
  ): Promise<SessionWorktreeInspection>;
  load(): Promise<SessionNavigationData>;
  move(
    sessionId: string,
    placement: SessionPlacement,
    orderedIds?: readonly string[],
  ): Promise<void>;
  pin(sessionId: string, pinned: boolean): Promise<void>;
  reorder(scope: NavigationOrderScope, orderedIds: readonly string[]): Promise<void>;
  subscribeChanged?(listener: () => void): Promise<() => void>;
}
export type SessionGitAction = 'create' | 'rename' | 'delete' | 'fetch' | 'merge';
export interface SessionGitActionInput {
  repositoryId: string;
  branchRef: string | null;
  action: SessionGitAction;
  value: string;
  startPoint: string;
}
export interface SessionGitPreview {
  path: string;
  arguments: readonly string[];
  status: string;
}
export interface SessionWorktreeInspection {
  path: string;
  branchRef: string | null;
  head: string;
  detached: boolean;
  staged: { files: number; bytes: number };
  unstaged: { files: number; bytes: number };
  untracked: { files: number; bytes: number };
}
export const emptySessionNavigation = (): SessionNavigationData => ({
  summaries: [],
  repositories: [],
  instances: [],
  owners: [],
  organization: [],
  orders: [],
});
