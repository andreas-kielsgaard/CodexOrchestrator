import type { SessionWorkflowTarget } from '../../application/agentSessions/workflowNavigation';
import {
  navigationFolders,
  navigationGroups,
  sessionContainers,
  navigationSiblings,
} from '../../application/agentSessions/navigationView';
import type { NavigationOrderScope } from '../../application/agentSessions/navigationOrder';
import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  SessionNavigationCommandRequest,
  SessionNavigationState,
} from '../../application/agentSessions/agentAccess';
import type {
  SessionNavigationModel,
  SessionNavigationSelection,
} from '../../application/agentSessions/navigation';
import type {
  SessionFolderTarget,
  SessionPlacement,
} from '../../application/agentSessions/organization';
import { formatSessionDeepLink } from '../../application/agentSessions/deepLinks';
import type { SessionNavigationController } from './useSessionNavigation';

export function useSessionNavigationCommands(options: {
  request?: SessionNavigationCommandRequest | null;
  complete?(id: string, state: SessionNavigationState | null, error: string | null): Promise<void>;
  model: SessionNavigationModel;
  tree: SessionNavigationController;
  selection: SessionNavigationSelection;
  collectionLoading: boolean;
  loading: boolean;
  error: string | null;
  onOpenWorkflow?(target: SessionWorkflowTarget): void;
  onSelect(id: string): void;
  onNew(target: SessionFolderTarget | null): void;
  onMove(id: string, placement: SessionPlacement, orderedIds?: readonly string[]): Promise<void>;
  onPin(id: string, pinned: boolean): Promise<void>;
  onReorder(scope: NavigationOrderScope, ids: readonly string[]): Promise<void>;
}) {
  const started = useRef<string | null>(null);
  const completed = useRef<string | null>(null);
  const [readyId, setReadyId] = useState<string | null>(null);
  const [settled, setSettled] = useState<{
    id: string;
    deeplink?: string;
    error?: string;
    workflow?: SessionWorkflowTarget;
  } | null>(null);
  const folderEntries = useMemo(() => navigationFolders(options.model), [options.model]);
  const folders = useMemo(() => folderEntries.map((f) => f.node), [folderEntries]);
  useEffect(() => {
    const request = options.request;
    if (!request || options.collectionLoading || request.id === started.current) return;
    started.current = request.id;
    void (async () => {
      const command = request.command;
      let deeplink: string | undefined;
      let workflow: SessionWorkflowTarget | undefined;
      if ('sessionId' in command && !options.model.sessions.has(command.sessionId))
        throw new Error('Session not found');
      if (
        'folderId' in command &&
        !folders.some((folder) => folder.id === command.folderId) &&
        !(command.kind === 'show_more' && command.folderId === 'unfiled')
      )
        throw new Error('Folder not found');
      switch (command.kind) {
        case 'inspect':
          break;
        case 'open_session':
          if (command.source === 'pinned') {
            if (!options.model.sessions.get(command.sessionId)?.pinned)
              throw new Error('Session is not pinned');
            options.tree.preserveDisclosureFor(command.sessionId);
          }
          options.onSelect(command.sessionId);
          break;
        case 'new_session':
          if (
            command.folderTarget &&
            !folders.some((folder) => sameFolder(folder.createTarget, command.folderTarget))
          )
            throw new Error('Folder not found');
          options.onNew(command.folderTarget);
          break;
        case 'set_folder_expanded':
          options.tree.setFolderExpanded(command.folderId, command.expanded);
          break;
        case 'set_group_expanded':
          if (!navigationGroups(options.model).some((g) => g.id === command.groupId))
            throw new Error('Group not found');
          options.tree.setFolderExpanded(command.groupId, command.expanded);
          break;
        case 'open_workflow':
          if (
            !options.onOpenWorkflow ||
            !folders.some(
              (f) =>
                f.createTarget?.kind === 'workflow_instance' &&
                f.createTarget.instanceId === command.instanceId,
            )
          )
            throw new Error('Workflow not available');
          workflow = { instanceId: command.instanceId };
          break;
        case 'open_session_workflow': {
          const owner = options.model.sessions.get(command.sessionId)?.owner;
          if (!owner || !options.onOpenWorkflow)
            throw new Error('Session has no available workflow');
          workflow = {
            instanceId: owner.instanceId,
            session: { nodeId: owner.nodeId, sessionId: command.sessionId },
          };
          break;
        }
        case 'show_more':
          options.tree.showMore(command.folderId);
          break;
        case 'move_session':
          await options.onMove(command.sessionId, command.placement, command.orderedIds);
          break;
        case 'pin_session':
          await options.onPin(command.sessionId, command.pinned);
          break;
        case 'reorder_navigation':
          await options.onReorder(command.scope, command.orderedIds);
          break;
        case 'get_deeplink':
          deeplink = formatSessionDeepLink(command.sessionId);
          break;
      }
      setSettled({ id: request.id, deeplink, workflow });
    })().catch((error) =>
      setSettled({ id: request.id, error: error instanceof Error ? error.message : String(error) }),
    );
  }, [options, folders]);
  useEffect(() => {
    if (!settled || !options.complete || completed.current === settled.id) return;
    if (readyId !== settled.id) {
      setReadyId(settled.id);
      return;
    }
    completed.current = settled.id;
    const state: SessionNavigationState = {
      selection: options.selection,
      loading: options.loading,
      error: options.error,
      folders: folderEntries.map(({ node: folder, parentId }) => ({
        parentId,
        role: folder.role,
        order: folder.order,
        orderedSiblingIds: navigationSiblings(options.model, folder.order.scope),
        id: folder.id,
        label: folder.label,
        expanded: options.tree.expanded.has(folder.id),
        createTarget: folder.createTarget,
      })),
      groups: navigationGroups(options.model).map((group) => ({
        ...group,
        expanded: options.tree.expanded.has(group.id),
      })),
      orders: sessionContainers(options.model).map((c) => ({
        scope: c.id === 'pinned' ? { kind: 'pinned' } : { kind: 'sessions', folderId: c.id },
        orderedIds: c.rows.map((r) => r.summary.id),
      })),
      openedWorkflow: settled.workflow,
      sessions: [...options.model.sessions.values()].map((row) => ({
        id: row.summary.id,
        title: row.summary.title,
        rowId: row.id,
        pinned: row.pinned,
        ownerLabel: row.ownerLabel,
        group: row.group,
      })),
      visibleRows: options.tree.rows.map((row) => ({
        id: row.id,
        kind: row.node.kind,
        level: row.level,
        parentId: row.parentId,
        ...(row.node.kind === 'session'
          ? { sessionId: row.node.summary.id }
          : row.node.kind === 'more'
            ? { folderId: row.node.folderId }
            : {}),
      })),
      ...(settled.deeplink ? { deeplink: settled.deeplink } : {}),
    };
    void options
      .complete(settled.id, settled.error ? null : state, settled.error ?? null)
      .then(() => {
        if (!settled.error && settled.workflow) options.onOpenWorkflow?.(settled.workflow);
      });
  }, [settled, readyId, options, folderEntries]);
}

function sameFolder(left: SessionFolderTarget | null, right: SessionFolderTarget | null): boolean {
  if (!left || !right) return left === right;
  return left.kind === 'repository' && right.kind === 'repository'
    ? left.repositoryId === right.repositoryId
    : left.kind === 'workflow_instance' &&
        right.kind === 'workflow_instance' &&
        left.instanceId === right.instanceId;
}
