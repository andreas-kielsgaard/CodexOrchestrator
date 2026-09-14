import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  SessionNavigationModel,
  SessionNavigationNode,
} from '../../application/agentSessions/navigation';
export interface VisibleSessionTreeRow {
  readonly id: string;
  readonly level: number;
  readonly parentId: string | null;
  readonly sectionId: string;
  readonly node:
    | SessionNavigationNode
    | {
        readonly kind: 'more';
        readonly folderId: string;
        readonly remaining: number;
        readonly label: string;
      };
}
export function visibleSessionRows(
  model: SessionNavigationModel,
  expanded: ReadonlySet<string>,
  shown: ReadonlySet<string>,
): VisibleSessionTreeRow[] {
  const rows: VisibleSessionTreeRow[] = [];
  function visit(
    nodes: readonly SessionNavigationNode[],
    parentId: string | null,
    sectionId: string,
    level: number,
    limitId: string,
    label: string,
    unlimited = false,
  ) {
    let sessions = 0,
      hidden = 0;
    for (const node of nodes) {
      if (node.kind === 'session' && ++sessions > 5 && !unlimited && !shown.has(limitId)) {
        hidden++;
        continue;
      }
      rows.push({ id: node.id, node, parentId, sectionId, level });
      if (node.kind === 'folder' && expanded.has(node.id))
        visit(node.children, node.id, sectionId, level + 1, node.id, node.label);
    }
    if (hidden)
      rows.push({
        id: `more:${limitId}`,
        node: { kind: 'more', folderId: limitId, remaining: hidden, label },
        parentId,
        sectionId,
        level,
      });
  }
  model.sections.forEach((section) =>
    visit(section.children, null, section.id, 1, section.id, section.label, section.unlimited),
  );
  return rows;
}
function ancestorPath(
  nodes: readonly SessionNavigationNode[],
  id: string,
  path: string[] = [],
): string[] | null {
  for (const node of nodes) {
    if (node.id === id) return path;
    if (node.kind === 'folder') {
      const found = ancestorPath(node.children, id, [...path, node.id]);
      if (found) return found;
    }
  }
  return null;
}
export function useSessionTree(
  model: SessionNavigationModel,
  selectedSessionId: string | null,
  revealKey: string,
) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [shown, setShown] = useState<ReadonlySet<string>>(new Set());
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const refs = useRef(new Map<string, HTMLElement>());
  const initialized = useRef(false);
  const lastReveal = useRef('');
  useEffect(() => {
    if (initialized.current) return;
    const ids: string[] = [];
    const collect = (nodes: readonly SessionNavigationNode[]) =>
      nodes.forEach((n) => {
        if (n.kind === 'folder') {
          ids.push(n.id);
          collect(n.children);
        }
      });
    model.sections.forEach((s) => collect(s.children));
    if (ids.length) {
      initialized.current = true;
      setExpanded(new Set(ids));
    }
  }, [model]);
  useEffect(() => {
    if (!selectedSessionId || lastReveal.current === revealKey) return;
    const row = model.sessions.get(selectedSessionId);
    if (!row) return;
    const section = model.sections.find((s) => ancestorPath(s.children, row.id) !== null);
    if (!section) return;
    const path = ancestorPath(section.children, row.id)!;
    lastReveal.current = revealKey;
    const revealedExpansion = new Set([...expanded, ...path]);
    setExpanded(revealedExpansion);
    if (!visibleSessionRows(model, revealedExpansion, shown).some((item) => item.id === row.id))
      setShown((current) => new Set([...current, path.at(-1) ?? section.id]));
    setFocusedId(row.id);
    requestAnimationFrame(() => refs.current.get(row.id)?.scrollIntoView({ block: 'nearest' }));
  }, [model, selectedSessionId, revealKey, expanded, shown]);
  const rows = useMemo(() => visibleSessionRows(model, expanded, shown), [model, expanded, shown]);
  const focus = (id: string | undefined) => {
    if (id) {
      setFocusedId(id);
      refs.current.get(id)?.focus();
    }
  };
  const toggle = (id: string) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const showMore = (folderId: string) => {
    const index = rows.findIndex((row) => row.id === `more:${folderId}`);
    const nextShown = new Set([...shown, folderId]);
    setShown(nextShown);
    const next = visibleSessionRows(model, expanded, nextShown)[index];
    if (next) {
      setFocusedId(next.id);
      requestAnimationFrame(() => refs.current.get(next.id)?.focus());
    }
  };
  const activeId = rows.some((r) => r.id === focusedId) ? focusedId : (rows[0]?.id ?? null);
  const setFolderExpanded = (id: string, value: boolean) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (value) next.add(id);
      else next.delete(id);
      return next;
    });
  return {
    rows,
    expanded,
    activeId,
    refs,
    focus,
    toggle,
    showMore,
    setFocusedId,
    setFolderExpanded,
  };
}
export type SessionTreeController = ReturnType<typeof useSessionTree>;
