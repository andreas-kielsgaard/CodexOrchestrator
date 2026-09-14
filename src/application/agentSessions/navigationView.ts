import type {
  SessionNavigationFolder,
  SessionNavigationModel,
  SessionNavigationNode,
  SessionNavigationRow,
} from './navigation';
import { orderScopeKey, type NavigationOrderScope } from './navigationOrder';

export interface NavigationEntry {
  readonly id: string;
  readonly level: number;
  readonly parentId: string | null;
  readonly sectionId: string;
  readonly node:
    | SessionNavigationNode
    | {
        readonly kind: 'group';
        readonly id: string;
        readonly label: string;
        readonly folderId: string;
        readonly group: 'added' | 'owned';
      }
    | {
        readonly kind: 'more';
        readonly folderId: string;
        readonly remaining: number;
        readonly label: string;
      };
  readonly children: readonly NavigationContent[];
}
export type NavigationContent = NavigationEntry;
export interface NavigationFolderEntry {
  readonly node: SessionNavigationFolder;
  readonly parentId: string | null;
  readonly sectionId: string;
}

export function navigationFolders(model: SessionNavigationModel): NavigationFolderEntry[] {
  const result: NavigationFolderEntry[] = [];
  const visit = (
    nodes: readonly SessionNavigationNode[],
    parentId: string | null,
    sectionId: string,
  ) => {
    for (const node of nodes)
      if (node.kind === 'folder') {
        result.push({ node, parentId, sectionId });
        visit(node.children, node.id, sectionId);
      }
  };
  model.sections.forEach((s) => visit(s.children, null, s.id));
  return result;
}

export function navigationSiblings(
  model: SessionNavigationModel,
  scope: NavigationOrderScope,
): string[] {
  if (scope.kind === 'pinned')
    return sessionContainers(model)
      .find((c) => c.id === 'pinned')!
      .rows.map((r) => r.summary.id);
  if (scope.kind === 'sessions')
    return (
      sessionContainers(model)
        .find((c) => c.id === scope.folderId)
        ?.rows.map((r) => r.summary.id) ?? []
    );
  return navigationFolders(model)
    .filter((f) => orderScopeKey(f.node.order.scope) === orderScopeKey(scope))
    .map((f) => f.node.order.id);
}

export function ancestorPath(
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

export function navigationGroups(model: SessionNavigationModel) {
  return navigationFolders(model)
    .filter((f) => f.node.role === 'workflow')
    .flatMap(({ node }) =>
      (['added', 'owned'] as const)
        .filter((group) => node.children.some((n) => n.kind === 'session' && n.group === group))
        .map((group) => ({
          id: `${node.id}:${group}`,
          folderId: node.id,
          group,
          label: group === 'added' ? 'Added sessions' : 'Workflow sessions',
        })),
    );
}

export function sessionContainers(model: SessionNavigationModel) {
  return [
    ...model.sections
      .filter((s) => s.id !== 'repositories')
      .map((s) => ({
        id: s.id,
        placement: { kind: 'unfiled' } as const,
        rows: s.children.filter((n): n is SessionNavigationRow => n.kind === 'session'),
      })),
    ...navigationFolders(model)
      .filter((f) => f.node.role === 'workflow' || f.node.id.endsWith(':sessions'))
      .map(({ node }) => ({
        id: node.id,
        placement: node.placement,
        rows: node.children.filter((n): n is SessionNavigationRow => n.kind === 'session'),
      })),
  ];
}

/** One projection supplies nested render surfaces and the keyboard/agent traversal. */
export function projectNavigationView(
  model: SessionNavigationModel,
  expanded: ReadonlySet<string>,
  shown: ReadonlySet<string>,
) {
  const rows: NavigationEntry[] = [];
  function visit(
    nodes: readonly SessionNavigationNode[],
    parentId: string | null,
    sectionId: string,
    level: number,
    limitId: string,
    label: string,
    unlimited = false,
  ): NavigationEntry[] {
    const content: NavigationEntry[] = [];
    let sessions = 0,
      hidden = 0;
    const entryFor = (
      node: NavigationEntry['node'],
      parent = parentId,
      depth = level,
    ): NavigationEntry => ({
      id: node.kind === 'more' ? `more:${node.folderId}` : node.id,
      node,
      parentId: parent,
      sectionId,
      level: depth,
      children: [],
    });
    const sessionEntry = (node: SessionNavigationRow, parent = parentId, depth = level) => {
      if (++sessions > 5 && !unlimited && !shown.has(limitId)) {
        hidden++;
        return null;
      }
      const entry = entryFor(node, parent, depth);
      rows.push(entry);
      return entry;
    };
    const visitedGroups = new Set<string>();
    for (const node of nodes) {
      if (node.kind === 'session' && node.group) {
        if (visitedGroups.has(node.group)) continue;
        visitedGroups.add(node.group);
        const group = {
          kind: 'group' as const,
          id: `${limitId}:${node.group}`,
          folderId: limitId,
          group: node.group,
          label: node.group === 'added' ? 'Added sessions' : 'Workflow sessions',
        };
        const entry = entryFor(group);
        rows.push(entry);
        const children = expanded.has(group.id)
          ? nodes
              .filter(
                (n): n is SessionNavigationRow => n.kind === 'session' && n.group === node.group,
              )
              .flatMap((n) => {
                const child = sessionEntry(n, group.id, level + 1);
                return child ? [child] : [];
              })
          : [];
        content.push({ ...entry, children });
      } else if (node.kind === 'session') {
        const entry = sessionEntry(node);
        if (entry) content.push(entry);
      } else {
        const entry = entryFor(node);
        rows.push(entry);
        content.push({
          ...entry,
          children: expanded.has(node.id)
            ? visit(node.children, node.id, sectionId, level + 1, node.id, node.label)
            : [],
        });
      }
    }
    if (hidden) {
      const entry = entryFor({ kind: 'more', folderId: limitId, remaining: hidden, label });
      content.push(entry);
      rows.push(entry);
    }
    return content;
  }
  const sections = model.sections.map((section) => ({
    ...section,
    content: visit(
      section.children,
      null,
      section.id,
      1,
      section.id,
      section.label,
      section.unlimited,
    ),
  }));
  return { sections, rows };
}
export const visibleSessionRows = (
  model: SessionNavigationModel,
  expanded: ReadonlySet<string>,
  shown: ReadonlySet<string>,
) => projectNavigationView(model, expanded, shown).rows;
