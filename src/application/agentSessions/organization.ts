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
  readonly summaries: readonly AgentSessionSummaryDto[];
  readonly repositories: readonly NavigationRepository[];
  readonly instances: readonly NavigationInstance[];
  readonly owners: readonly WorkflowSessionOwner[];
  readonly organization: readonly SessionOrganization[];
}
export interface SessionNavigationClient {
  load(): Promise<SessionNavigationData>;
  move(sessionId: string, placement: SessionPlacement): Promise<void>;
  pin(sessionId: string, pinned: boolean): Promise<void>;
  subscribeChanged?(listener: () => void): Promise<() => void>;
}
export const emptySessionNavigation = (): SessionNavigationData => ({
  summaries: [],
  repositories: [],
  instances: [],
  owners: [],
  organization: [],
});
