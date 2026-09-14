import type { SessionFolderTarget, SessionPlacement } from './organization';
import type { SessionNavigationSelection } from './navigation';

export type SessionNavigationCommand =
  | { kind: 'inspect' }
  | { kind: 'open_session'; sessionId: string }
  | { kind: 'new_session'; folderTarget: SessionFolderTarget | null }
  | { kind: 'set_folder_expanded'; folderId: string; expanded: boolean }
  | { kind: 'show_more'; folderId: string }
  | { kind: 'move_session'; sessionId: string; placement: SessionPlacement }
  | { kind: 'pin_session'; sessionId: string; pinned: boolean }
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
    sessionId?: string;
    folderId?: string;
  }[];
  deeplink?: string;
}
export interface SessionNavigationAgentAccess {
  subscribe(listener: (request: SessionNavigationCommandRequest) => void): Promise<() => void>;
  complete(id: string, state: SessionNavigationState | null, error: string | null): Promise<void>;
}
