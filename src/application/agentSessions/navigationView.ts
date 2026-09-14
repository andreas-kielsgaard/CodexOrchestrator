import type {
  SessionNavigationFolder,
  SessionNavigationModel,
  SessionNavigationNode,
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
        readonly kind: 'more';
        readonly folderId: string;
        readonly remaining: number;
        readonly label: string;
      };
  readonly children: readonly NavigationContent[];
}
export type NavigationContent =
  | NavigationEntry
  | {
      readonly kind: 'group';
      readonly id: string;
      readonly label: string;
      readonly children: readonly NavigationEntry[];
    };
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
  ): NavigationContent[] {
    const content: NavigationContent[] = [];
    let sessions = 0,
      hidden = 0;
    for (const node of nodes) {
      if (node.kind === 'session' && ++sessions > 5 && !unlimited && !shown.has(limitId)) {
        hidden++;
        continue;
      }
      const entry: NavigationEntry = {
        id: node.id,
        node,
        parentId,
        sectionId,
        level,
        children: [],
      };
      rows.push(entry);
      if (node.kind === 'folder' && expanded.has(node.id)) {
        const children = visit(node.children, node.id, sectionId, level + 1, node.id, node.label);
        content.push({ ...entry, children });
      } else if (node.kind === 'session' && node.group) {
        const previous = content.at(-1);
        if (previous && 'kind' in previous && previous.id === `${limitId}:${node.group}`) {
          content[content.length - 1] = { ...previous, children: [...previous.children, entry] };
        } else
          content.push({
            kind: 'group',
            id: `${limitId}:${node.group}`,
            label: node.group === 'added' ? 'Added sessions' : 'Workflow sessions',
            children: [entry],
          });
      } else content.push(entry);
    }
    if (hidden) {
      const entry: NavigationEntry = {
        id: `more:${limitId}`,
        node: { kind: 'more', folderId: limitId, remaining: hidden, label },
        parentId,
        sectionId,
        level,
        children: [],
      };
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
