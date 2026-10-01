import { useRef, useState, type PointerEvent } from 'react';
import type {
  SessionNavigationFolder,
  SessionNavigationModel,
  SessionNavigationRow,
} from '../../application/agentSessions/navigation';
import { targetId } from '../../application/agentSessions/navigation';
import type { SessionPlacement } from '../../application/agentSessions/organization';
import {
  insertNavigationSibling,
  orderScopeKey,
  type NavigationOrderItem,
  type NavigationOrderScope,
} from '../../application/agentSessions/navigationOrder';
import {
  navigationFolders,
  navigationSiblings,
  sessionContainers,
} from '../../application/agentSessions/navigationView';

type DragSource =
  { kind: 'session'; id: string } | { kind: 'container'; item: NavigationOrderItem };
type DropTarget =
  | {
      kind: 'layout';
      id: string;
      side: 'before' | 'after';
      scope: NavigationOrderScope;
      orderedIds: readonly string[];
      placement?: SessionPlacement;
    }
  | {
      kind: 'session';
      id: string;
      side: 'before' | 'after';
      placement: SessionPlacement;
      scope: NavigationOrderScope;
      orderedIds: readonly string[];
    }
  | { kind: 'container'; id: string; side: 'before' | 'after'; item: NavigationOrderItem };

/** Pointer capture keeps sidebar gestures inside the application, including on WebView2. */
export function useSessionNavigationDrag(
  model: SessionNavigationModel,
  enabled: boolean,
  onMove: (
    id: string,
    placement: SessionPlacement,
    orderedIds?: readonly string[],
  ) => Promise<void>,
  onReorder: (scope: NavigationOrderScope, ids: readonly string[]) => Promise<void>,
) {
  const gesture = useRef<{ source: DragSource; x: number; y: number; dragging: boolean } | null>(
    null,
  );
  const suppressClick = useRef(false);
  const [indicator, setIndicator] = useState<DropTarget | null>(null);
  const folders = navigationFolders(model);
  const targetAt = (source: DragSource, x: number, y: number): DropTarget | null => {
    const element = document.elementFromPoint(x, y);
    const layoutSurface = element?.closest<HTMLElement>('[data-session-layout]');
    const sourceScope =
      source.kind === 'container'
        ? source.item.scope
        : model.sessions.get(source.id)?.layout?.scope;
    const destinationFolder = folders.find(
      (folder) => folder.node.id === layoutSurface?.dataset.sessionLayout,
    )?.node;
    const layoutScope: NavigationOrderScope | undefined =
      destinationFolder &&
      destinationFolder.children.some((node) => node.kind === 'folder' && node.role === 'branch')
        ? { kind: 'session_layout', folderId: destinationFolder.id }
        : undefined;
    if (
      layoutSurface &&
      layoutScope?.kind === 'session_layout' &&
      (source.kind === 'session' || sourceScope?.kind === 'session_layout')
    ) {
      if (source.kind === 'container' && orderScopeKey(sourceScope!) !== orderScopeKey(layoutScope))
        return null;
      const sourceId = source.kind === 'container' ? source.item.id : `session:${source.id}`;
      const item = element?.closest<HTMLElement>('[data-branch-layout-item]');
      const targetId = item?.dataset.branchLayoutItem;
      const rect = item?.getBoundingClientRect();
      const branch = folders.find(
        (folder) => folder.node.id === targetId && folder.node.role === 'branch',
      )?.node;
      const ids = navigationSiblings(model, layoutScope);
      if (branch && source.kind === 'session' && rect && y > rect.top + 7 && y < rect.bottom - 7) {
        if (model.sessions.get(source.id)?.branch?.id !== branch.id) return null;
        return {
          kind: 'layout',
          id: branch.id,
          side: 'after',
          scope: layoutScope,
          orderedIds: ids.filter((id) => id !== sourceId),
        };
      }
      const anchor = targetId ?? ids.at(-1);
      if (!anchor || anchor === sourceId) return null;
      const side = rect && y < rect.top + rect.height / 2 ? 'before' : 'after';
      return {
        kind: 'layout',
        id:
          item?.querySelector<HTMLElement>('[data-navigation-header]')?.dataset.navigationHeader ??
          (source.kind === 'session' && targetId?.startsWith('session:')
            ? model.sessions.get(targetId.slice(8))?.id
            : targetId) ??
          layoutScope.folderId,
        side,
        scope: layoutScope,
        orderedIds: insertNavigationSibling(ids, sourceId, anchor, side),
        placement:
          source.kind === 'session' &&
          (!sourceScope || orderScopeKey(sourceScope) !== orderScopeKey(layoutScope))
            ? destinationFolder?.placement
            : undefined,
      };
    }
    if (source.kind === 'container' && source.item.scope.kind === 'session_layout') return null;
    if (source.kind === 'session') {
      const surface = element?.closest<HTMLElement>('[data-session-placement]');
      if (!surface) return null;
      const surfaceId = surface.dataset.sessionPlacement!;
      const folder = folders.find((f) => f.node.id === surfaceId)?.node;
      const containerId = folder ? targetId(folder.placement) : surfaceId;
      const destinationSurface =
        [...document.querySelectorAll<HTMLElement>('[data-session-placement]')].find(
          (e) => e.dataset.sessionPlacement === containerId,
        ) ?? surface;
      const container = sessionContainers(model).find((c) => c.id === containerId);
      const session = model.sessions.get(source.id);
      if (!container || !session || (containerId === 'pinned' && !session.pinned)) return null;
      const group = containerId.startsWith('instance:')
        ? containerId === `instance:${session.owner?.instanceId}`
          ? 'owned'
          : 'added'
        : undefined;
      const eligible = container.rows.filter(
        (r) => r.summary.id !== source.id && r.group === group,
      );
      const eligibleIds = new Set(eligible.map((r) => r.id));
      const visible = [
        ...destinationSurface.querySelectorAll<HTMLElement>('[data-session-row]'),
      ].filter((e) => eligibleIds.has(e.dataset.sessionRow!));
      const next = visible.find((e) => {
        const bounds = e.getBoundingClientRect();
        return y < bounds.top + bounds.height / 2;
      });
      const anchor = next ?? visible.at(-1);
      const side = next ? 'before' : 'after';
      const ids = container.rows.map((r) => r.summary.id);
      const target = anchor ? eligible.find((r) => r.id === anchor.dataset.sessionRow) : undefined;
      let orderedIds: string[];
      if (target) orderedIds = insertNavigationSibling(ids, source.id, target.summary.id, side);
      else {
        orderedIds = ids.filter((id) => id !== source.id);
        const first = eligible[0];
        const position = first
          ? orderedIds.indexOf(first.summary.id)
          : group === 'added'
            ? 0
            : orderedIds.length;
        orderedIds.splice(position, 0, source.id);
      }
      const groupHeader = group
        ? destinationSurface.querySelector<HTMLElement>(`[data-session-group="${group}"]`)
        : null;
      return {
        kind: 'session',
        id:
          anchor?.dataset.sessionRow ??
          groupHeader?.id ??
          destinationSurface.dataset.sessionPlacement!,
        side: anchor ? side : 'after',
        placement: container.placement,
        scope:
          containerId === 'pinned'
            ? { kind: 'pinned' }
            : { kind: 'sessions', folderId: containerId },
        orderedIds,
      };
    }
    const header = element?.closest<HTMLElement>('[data-navigation-header]');
    const folder = folders.find((f) => f.node.id === header?.dataset.navigationHeader)?.node;
    if (
      !header ||
      !folder ||
      source.item.id === folder.order.id ||
      orderScopeKey(source.item.scope) !== orderScopeKey(folder.order.scope)
    )
      return null;
    const rect = header.getBoundingClientRect();
    return {
      kind: 'container',
      id: folder.id,
      item: folder.order,
      side: y < rect.top + rect.height / 2 ? 'before' : 'after',
    };
  };
  const clear = () => {
    gesture.current = null;
    setIndicator(null);
  };
  const release = (event: PointerEvent<HTMLElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    clear();
    setTimeout(() => {
      suppressClick.current = false;
    }, 0);
  };
  const sourceProps = (source: DragSource) => ({
    draggable: false,
    'data-draggable': enabled || undefined,
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (!enabled || event.button !== 0 || (event.target as Element).closest('button')) return;
      event.stopPropagation();
      gesture.current = { source, x: event.clientX, y: event.clientY, dragging: false };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const pending = gesture.current;
      if (!pending) return;
      if (!pending.dragging && Math.hypot(event.clientX - pending.x, event.clientY - pending.y) < 5)
        return;
      pending.dragging = true;
      suppressClick.current = true;
      event.preventDefault();
      event.stopPropagation();
      setIndicator(targetAt(pending.source, event.clientX, event.clientY));
    },
    onPointerUp: (event: PointerEvent<HTMLElement>) => {
      const pending = gesture.current;
      if (pending?.dragging) {
        event.preventDefault();
        event.stopPropagation();
        const target = targetAt(pending.source, event.clientX, event.clientY);
        if (target?.kind === 'layout') {
          void (async () => {
            if (target.placement && pending.source.kind === 'session')
              await onMove(pending.source.id, target.placement);
            await onReorder(target.scope, target.orderedIds);
          })();
        } else if (pending.source.kind === 'session' && target?.kind === 'session') {
          if (target.scope.kind === 'pinned') void onReorder(target.scope, target.orderedIds);
          else void onMove(pending.source.id, target.placement, target.orderedIds);
        } else if (pending.source.kind === 'container' && target?.kind === 'container') {
          void onReorder(
            target.item.scope,
            insertNavigationSibling(
              navigationSiblings(model, target.item.scope),
              pending.source.item.id,
              target.item.id,
              target.side,
            ),
          );
        }
      }
      release(event);
    },
    onPointerCancel: release,
    onLostPointerCapture: clear,
    onKeyDownCapture: (event: React.KeyboardEvent<HTMLElement>) => {
      if (event.key === 'Escape' && gesture.current) {
        event.stopPropagation();
        clear();
      }
    },
  });
  return {
    indicator,
    sessionProps: (row: SessionNavigationRow) => ({
      ...sourceProps({ kind: 'session', id: row.summary.id }),
      'data-session-row': row.id,
    }),
    headerProps: (folder: SessionNavigationFolder) => ({
      ...sourceProps({ kind: 'container', item: folder.order }),
      'data-navigation-header': folder.id,
    }),
    dropProps: (id: string) => ({ 'data-session-placement': id }),
    consumeClick: () => {
      const suppressed = suppressClick.current;
      suppressClick.current = false;
      return suppressed;
    },
  };
}
export type SessionNavigationDrag = ReturnType<typeof useSessionNavigationDrag>;
