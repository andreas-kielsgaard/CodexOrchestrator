import { useEffect, useMemo, useRef, useState } from 'react';
import type { SessionNavigationModel } from '../../application/agentSessions/navigation';
import {
  ancestorPath,
  navigationFolders,
  navigationGroups,
  projectNavigationView,
  visibleSessionRows,
} from '../../application/agentSessions/navigationView';
export function useSessionNavigation(
  model: SessionNavigationModel,
  selectedSessionId: string | null,
  revealKey: string,
) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [shown, setShown] = useState<ReadonlySet<string>>(new Set());
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const refs = useRef(new Map<string, HTMLElement>());
  const initialized = useRef(false);
  const knownGroups = useRef(new Set<string>());
  const lastReveal = useRef('');
  const preserveDisclosure = useRef<string | null>(null);
  useEffect(() => {
    if (initialized.current) return;
    const ids = [
      ...navigationFolders(model).map((f) => f.node.id),
      ...navigationGroups(model).map((g) => g.id),
    ];
    if (ids.length) {
      initialized.current = true;
      setExpanded(new Set(ids));
    }
  }, [model]);
  useEffect(() => {
    const ids = navigationGroups(model).map((g) => g.id);
    const added = ids.filter((id) => !knownGroups.current.has(id));
    knownGroups.current = new Set(ids);
    if (added.length) setExpanded((current) => new Set([...current, ...added]));
  }, [model]);
  useEffect(() => {
    if (!selectedSessionId || lastReveal.current === revealKey) return;
    const row = model.sessions.get(selectedSessionId);
    if (!row) return;
    const section = model.sections.find((s) => ancestorPath(s.children, row.id) !== null);
    if (!section) return;
    const path = ancestorPath(section.children, row.id)!;
    lastReveal.current = revealKey;
    if (preserveDisclosure.current === selectedSessionId) {
      preserveDisclosure.current = null;
      setFocusedId(`pinned:session:${selectedSessionId}`);
      return;
    }
    preserveDisclosure.current = null;
    if (row.group) path.push(`${path.at(-1)}:${row.group}`);
    const revealedExpansion = new Set([...expanded, ...path]);
    setExpanded(revealedExpansion);
    if (!visibleSessionRows(model, revealedExpansion, shown).some((item) => item.id === row.id))
      setShown(
        (current) => new Set([...current, (row.group ? path.at(-2) : path.at(-1)) ?? section.id]),
      );
    setFocusedId(row.id);
    requestAnimationFrame(() => refs.current.get(row.id)?.scrollIntoView({ block: 'nearest' }));
  }, [model, selectedSessionId, revealKey, expanded, shown]);
  const view = useMemo(
    () => projectNavigationView(model, expanded, shown),
    [model, expanded, shown],
  );
  const rows = view.rows;
  const previousRows = useRef(rows);
  useEffect(() => {
    if (focusedId && !rows.some((r) => r.id === focusedId)) {
      const previous = previousRows.current.find((r) => r.id === focusedId);
      const original =
        previous?.node.kind === 'session'
          ? model.sessions.get(previous.node.summary.id)?.id
          : undefined;
      const next =
        rows.find((r) => r.id === original) ??
        rows.find((r) => r.id === previous?.parentId) ??
        rows[0];
      setFocusedId(next?.id ?? null);
      if (next && document.activeElement === document.body) refs.current.get(next.id)?.focus();
    }
    previousRows.current = rows;
  }, [rows, focusedId, model]);
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
    const previousIds = new Set(rows.map((row) => row.id));
    const nextShown = new Set([...shown, folderId]);
    setShown(nextShown);
    const next = visibleSessionRows(model, expanded, nextShown).find(
      (row) => row.node.kind === 'session' && !previousIds.has(row.id),
    );
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
  const selectedAncestors = new Set<string>();
  const selected = selectedSessionId ? model.sessions.get(selectedSessionId) : undefined;
  if (selected)
    for (const section of model.sections) {
      const path = ancestorPath(section.children, selected.id);
      if (path) {
        path.forEach((id) => selectedAncestors.add(id));
        if (selected.group) selectedAncestors.add(`${path.at(-1)}:${selected.group}`);
      }
    }
  return {
    preserveDisclosureFor: (id: string) => {
      preserveDisclosure.current = id;
    },
    selectedAncestors,
    rows,
    view,
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
export type SessionNavigationController = ReturnType<typeof useSessionNavigation>;
