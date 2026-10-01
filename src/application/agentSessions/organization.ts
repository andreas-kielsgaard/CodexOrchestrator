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
  readonly id: string;
  readonly repositoryId: string;
  readonly branchRef: string;
  readonly createdAt: string | null;
  readonly paths: readonly string[];
}
export interface SessionNavigationClient {
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
export const emptySessionNavigation = (): SessionNavigationData => ({
  summaries: [],
  repositories: [],
  instances: [],
  owners: [],
  organization: [],
  orders: [],
});
