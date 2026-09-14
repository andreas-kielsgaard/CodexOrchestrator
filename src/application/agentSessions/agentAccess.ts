import type { SessionWorkflowTarget } from './workflowNavigation';
import type { NavigationOrderItem, NavigationOrderScope } from './navigationOrder';
import type { SessionFolderTarget, SessionPlacement } from './organization';
import type { SessionNavigationSelection } from './navigation';

export type SessionNavigationCommand =
  | { kind: 'inspect' }
  | { kind: 'open_session'; sessionId: string; source?: 'pinned' }
  | { kind: 'new_session'; folderTarget: SessionFolderTarget | null }
  | { kind: 'set_folder_expanded'; folderId: string; expanded: boolean }
  | { kind: 'set_group_expanded'; groupId: string; expanded: boolean }
  | { kind: 'open_workflow'; instanceId: string }
  | { kind: 'open_session_workflow'; sessionId: string }
  | { kind: 'show_more'; folderId: string }
  | {
      kind: 'move_session';
      sessionId: string;
      placement: SessionPlacement;
      orderedIds?: readonly string[];
    }
  | { kind: 'pin_session'; sessionId: string; pinned: boolean }
  | { kind: 'reorder_navigation'; scope: NavigationOrderScope; orderedIds: readonly string[] }
  | { kind: 'get_deeplink'; sessionId: string };
export interface SessionNavigationCommandRequest {
  id: string;
  command: SessionNavigationCommand;
}
export interface SessionNavigationState {
  selection: SessionNavigationSelection;
  loading: boolean;
  error: string | null;
  folders: readonly {
    id: string;
    label: string;
    expanded: boolean;
    parentId: string | null;
    role: 'repository' | 'section' | 'workflow';
    order: NavigationOrderItem;
    orderedSiblingIds: readonly string[];
    createTarget: SessionFolderTarget | null;
  }[];
  sessions: readonly {
    id: string;
    title: string;
    rowId: string;
    pinned: boolean;
    ownerLabel?: string;
    group?: string;
  }[];
  visibleRows: readonly {
    id: string;
    kind: string;
    level: number;
    parentId: string | null;
    sessionId?: string;
    folderId?: string;
  }[];
  groups: readonly { id: string; label: string; folderId: string; expanded: boolean }[];
  orders: readonly { scope: NavigationOrderScope; orderedIds: readonly string[] }[];
  openedWorkflow?: SessionWorkflowTarget;
  deeplink?: string;
}
export interface SessionNavigationAgentAccess {
  subscribe(listener: (request: SessionNavigationCommandRequest) => void): Promise<() => void>;
  complete(id: string, state: SessionNavigationState | null, error: string | null): Promise<void>;
}
