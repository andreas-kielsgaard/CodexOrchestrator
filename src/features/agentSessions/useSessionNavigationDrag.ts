import { useRef, useState, type PointerEvent } from 'react';
import type {
  SessionNavigationFolder,
  SessionNavigationModel,
} from '../../application/agentSessions/navigation';
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
} from '../../application/agentSessions/navigationView';

type DragSource =
  { kind: 'session'; id: string } | { kind: 'container'; item: NavigationOrderItem };
type DropTarget =
  | { id: string; side: 'inside'; placement: SessionPlacement }
  | { id: string; side: 'before' | 'after'; item: NavigationOrderItem };

/** Pointer capture keeps sidebar gestures inside the application, including on WebView2. */
export function useSessionNavigationDrag(
  model: SessionNavigationModel,
  enabled: boolean,
  onMove: (id: string, placement: SessionPlacement) => Promise<void>,
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
    if (source.kind === 'session') {
      const id = element?.closest<HTMLElement>('[data-session-placement]')?.dataset
        .sessionPlacement;
      if (id === 'unfiled') return { id, side: 'inside', placement: { kind: 'unfiled' } };
      const folder = folders.find((f) => f.node.id === id)?.node;
      return folder ? { id: folder.id, side: 'inside', placement: folder.placement } : null;
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
        if (pending.source.kind === 'session' && target?.side === 'inside')
          void onMove(pending.source.id, target.placement);
        else if (pending.source.kind === 'container' && target && target.side !== 'inside') {
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
    sessionProps: (id: string) => sourceProps({ kind: 'session', id }),
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
