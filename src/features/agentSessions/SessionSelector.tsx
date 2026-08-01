import {
  Bot,
  ChevronDown,
  ChevronRight,
  Folder,
  MessageSquarePlus,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
} from 'lucide-react';
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type RefObject,
} from 'react';
import type {
  AgentSessionNavigationModel,
  AgentSessionNavigationNode,
  AgentSessionNavigationSection,
  AgentSessionNavigationSession,
} from '../../application/agentSessionNavigation';

interface SessionSelectorProps {
  model: AgentSessionNavigationModel;
  selectedSessionId: string | null;
  expandedNodeIds: ReadonlySet<string>;
  loading: boolean;
  onExpandedNodeIdsChange(ids: ReadonlySet<string>): void;
  onSelect(sessionId: string): void;
  onNew(): void;
  onReload(): void;
}

interface VisibleTreeItem {
  readonly node: AgentSessionNavigationNode;
  readonly level: number;
  readonly parentId: string | null;
  readonly sectionId: string;
}

export function SessionSelector({
  model,
  selectedSessionId,
  expandedNodeIds,
  loading,
  onExpandedNodeIdsChange,
  onSelect,
  onNew,
  onReload,
}: SessionSelectorProps) {
  const [treeOpen, setTreeOpen] = useState(true);
  const selectedNodeId = selectedSessionId ? `session:${selectedSessionId}` : null;
  const visible = useMemo(
    () =>
      model.sections.flatMap((section) =>
        flattenVisibleTree(section.children, expandedNodeIds, 1, null, section.id),
      ),
    [expandedNodeIds, model.sections],
  );
  const [focusedId, setFocusedId] = useState<string | null>(selectedNodeId);
  const itemRefs = useRef(new Map<string, HTMLButtonElement>());
  const lastSelectedNodeId = useRef<string | null>(null);

  useEffect(() => {
    if (!selectedNodeId || lastSelectedNodeId.current === selectedNodeId) return;
    if (!model.sections.some((section) => findNode(section.children, selectedNodeId))) return;
    lastSelectedNodeId.current = selectedNodeId;
    const ancestors = findAncestorsInSections(model.sections, selectedNodeId);
    if (ancestors.some((id) => !expandedNodeIds.has(id)))
      onExpandedNodeIdsChange(new Set([...expandedNodeIds, ...ancestors]));
  }, [expandedNodeIds, model.sections, onExpandedNodeIdsChange, selectedNodeId]);

  useEffect(() => {
    if (!selectedNodeId) return;
    setFocusedId(selectedNodeId);
  }, [selectedNodeId]);

  useEffect(() => {
    if (focusedId && visible.some(({ node }) => node.id === focusedId)) return;
    const selectedAncestor = selectedNodeId
      ? findAncestorsInSections(model.sections, selectedNodeId)
          .reverse()
          .find((id) => visible.some(({ node }) => node.id === id))
      : undefined;
    setFocusedId(
      visible.some(({ node }) => node.id === selectedNodeId)
        ? selectedNodeId
        : (selectedAncestor ?? visible[0]?.node.id ?? null),
    );
  }, [focusedId, model.sections, selectedNodeId, visible]);

  const focus = (id: string | undefined) => {
    if (!id) return;
    setFocusedId(id);
    itemRefs.current.get(id)?.focus();
  };
  const toggle = (id: string, expanded?: boolean) => {
    const next = new Set(expandedNodeIds);
    const shouldExpand = expanded ?? !next.has(id);
    if (shouldExpand) next.add(id);
    else {
      next.delete(id);
      if (selectedNodeId && nodeContains(model.sections, id, selectedNodeId)) focus(id);
    }
    onExpandedNodeIdsChange(next);
  };
  const onTreeKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = visible.findIndex(({ node }) => node.id === focusedId);
    if (index < 0) return;
    const current = visible[index];
    if (event.key === 'ArrowDown') focus(visible[index + 1]?.node.id);
    else if (event.key === 'ArrowUp') focus(visible[index - 1]?.node.id);
    else if (event.key === 'Home') focus(visible[0]?.node.id);
    else if (event.key === 'End') focus(visible.at(-1)?.node.id);
    else if (event.key === 'ArrowRight' && current.node.kind === 'folder') {
      if (!expandedNodeIds.has(current.node.id)) toggle(current.node.id, true);
      else if (visible[index + 1]?.level === current.level + 1) focus(visible[index + 1].node.id);
    } else if (event.key === 'ArrowLeft') {
      if (current.node.kind === 'folder' && expandedNodeIds.has(current.node.id))
        toggle(current.node.id, false);
      else focus(current.parentId ?? undefined);
    } else if (event.key === 'Enter' || event.key === ' ') {
      if (current.node.kind === 'folder') toggle(current.node.id);
      else onSelect(current.node.summary.id);
    } else return;
    event.preventDefault();
  };

  return (
    <nav
      className={`agent-session-selector${treeOpen ? ' is-open' : ''}`}
      aria-label="Session list"
    >
      <header>
        <div>
          <p className="eyebrow">Workspace</p>
          <h1>Agent Sessions</h1>
        </div>
        <button className="icon-button" type="button" onClick={onNew} aria-label="New session">
          <MessageSquarePlus size={17} aria-hidden="true" />
        </button>
      </header>
      <button
        className="session-tree-toggle"
        type="button"
        aria-expanded={treeOpen}
        aria-controls="agent-session-tree"
        onClick={() => setTreeOpen((current) => !current)}
      >
        {treeOpen ? (
          <PanelLeftClose size={16} aria-hidden="true" />
        ) : (
          <PanelLeftOpen size={16} aria-hidden="true" />
        )}
        {treeOpen ? 'Hide sessions' : 'Browse sessions'}
      </button>
      <button className="session-new-button" type="button" onClick={onNew}>
        New session
      </button>
      <div id="agent-session-tree" className="session-tree">
        {model.sections.map((section) => (
          <SessionTreeSection
            key={section.id}
            section={section}
            visible={visible.filter(({ sectionId }) => sectionId === section.id)}
            expandedNodeIds={expandedNodeIds}
            selectedSessionId={selectedSessionId}
            selectedNodeId={selectedNodeId}
            focusedId={focusedId}
            loading={loading}
            itemRefs={itemRefs}
            onFocus={setFocusedId}
            onKeyDown={onTreeKeyDown}
            onToggle={toggle}
            onSelect={onSelect}
          />
        ))}
      </div>
      <button
        className="session-refresh-button"
        type="button"
        onClick={onReload}
        disabled={loading}
      >
        <RefreshCw className={loading ? 'spin' : ''} size={15} aria-hidden="true" />
        Refresh
      </button>
    </nav>
  );
}

function SessionTreeSection({
  section,
  visible,
  expandedNodeIds,
  selectedSessionId,
  selectedNodeId,
  focusedId,
  loading,
  itemRefs,
  onFocus,
  onKeyDown,
  onToggle,
  onSelect,
}: {
  readonly section: AgentSessionNavigationSection;
  readonly visible: readonly VisibleTreeItem[];
  readonly expandedNodeIds: ReadonlySet<string>;
  readonly selectedSessionId: string | null;
  readonly selectedNodeId: string | null;
  readonly focusedId: string | null;
  readonly loading: boolean;
  readonly itemRefs: RefObject<Map<string, HTMLButtonElement>>;
  readonly onFocus: (id: string) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void;
  readonly onToggle: (id: string) => void;
  readonly onSelect: (sessionId: string) => void;
}) {
  const headingId = `session-section-${section.id}`;
  return (
    <section className="session-tree-section" aria-labelledby={headingId}>
      <h2 id={headingId}>{section.label}</h2>
      <div
        role="tree"
        aria-label={`${section.label} session hierarchy`}
        aria-busy={loading}
        onKeyDown={onKeyDown}
      >
        {visible.length === 0 ? (
          <p className="session-list-empty">
            {section.id === 'independent' ? 'No independent Sessions.' : 'No Epic Sessions.'}
          </p>
        ) : (
          visible.map(({ node, level }) => {
            const folder = node.kind === 'folder';
            const selected = !folder && node.summary.id === selectedSessionId;
            const containsSelected =
              folder &&
              Boolean(selectedNodeId) &&
              nodeContains([section], node.id, selectedNodeId!);
            return (
              <button
                ref={(element) => {
                  if (element) itemRefs.current?.set(node.id, element);
                  else itemRefs.current?.delete(node.id);
                }}
                className={`session-tree-item session-tree-item--${node.kind}${selected ? ' is-selected' : ''}${containsSelected ? ' has-selected-descendant' : ''}`}
                style={{ '--tree-level': level } as CSSProperties}
                role="treeitem"
                type="button"
                key={node.id}
                tabIndex={focusedId === node.id ? 0 : -1}
                aria-level={level}
                aria-expanded={folder ? expandedNodeIds.has(node.id) : undefined}
                aria-selected={folder ? undefined : selected}
                onFocus={() => onFocus(node.id)}
                onClick={() => {
                  if (node.kind === 'folder') onToggle(node.id);
                  else onSelect(node.summary.id);
                }}
              >
                {node.kind === 'folder' ? (
                  <>
                    {expandedNodeIds.has(node.id) ? (
                      <ChevronDown
                        className="session-tree-item__chevron"
                        size={14}
                        aria-hidden="true"
                      />
                    ) : (
                      <ChevronRight
                        className="session-tree-item__chevron"
                        size={14}
                        aria-hidden="true"
                      />
                    )}
                    <Folder className="session-tree-item__folder" size={15} aria-hidden="true" />
                    <span className="session-tree-item__folder-label">{node.label}</span>
                    {containsSelected ? (
                      <span className="session-tree-item__selected-descendant">
                        Contains selected Session
                      </span>
                    ) : null}
                  </>
                ) : (
                  <SessionTreeLabel session={node} />
                )}
              </button>
            );
          })
        )}
      </div>
    </section>
  );
}

function SessionTreeLabel({ session }: { readonly session: AgentSessionNavigationSession }) {
  const status = sessionStatus(session);
  const role = session.identity?.harnessRole ?? session.relationshipRoles.join(' · ');
  return (
    <>
      <span
        className="session-tree-item__identity"
        style={
          session.identity?.visualIdentity
            ? ({
                '--identity-accent': session.identity.visualIdentity.accentColor,
              } as CSSProperties)
            : undefined
        }
      >
        {session.identity ? (
          session.identity.agentName.slice(0, 1).toUpperCase()
        ) : (
          <Bot size={14} aria-hidden="true" />
        )}
      </span>
      <span className="session-tree-item__copy">
        <strong>
          {session.identity
            ? `${session.identity.agentName}: ${session.summary.title}`
            : session.summary.title}
        </strong>
        <small>
          {session.identity
            ? `${session.identity.agentName}: ${session.identity.harnessRole}`
            : role || 'Independent Agent Session'}
        </small>
      </span>
      <span className={`session-tree-item__status session-tree-item__status--${status.kind}`}>
        <span aria-hidden="true" />
        {status.label}
      </span>
    </>
  );
}

function sessionStatus(session: AgentSessionNavigationSession) {
  const status = session.summary.latestInvocationStatus;
  if (status === 'pending') return { kind: 'active', label: 'Starting' };
  if (status === 'running' || session.summary.hasActiveInvocation)
    return { kind: 'active', label: 'Processing' };
  if (status === 'completed') return { kind: 'completed', label: 'Completed' };
  if (status === 'failed') return { kind: 'attention', label: 'Failed' };
  if (status === 'canceled') return { kind: 'quiet', label: 'Canceled' };
  if (status === 'interrupted') return { kind: 'attention', label: 'Interrupted' };
  return { kind: 'quiet', label: formatDate(session.summary.updatedAt) };
}

function flattenVisibleTree(
  nodes: readonly AgentSessionNavigationNode[],
  expanded: ReadonlySet<string>,
  level = 1,
  parentId: string | null = null,
  sectionId: string,
): VisibleTreeItem[] {
  return nodes.flatMap((node) => [
    { node, level, parentId, sectionId },
    ...(node.kind === 'folder' && expanded.has(node.id)
      ? flattenVisibleTree(node.children, expanded, level + 1, node.id, sectionId)
      : []),
  ]);
}

function findAncestorsInSections(
  sections: readonly AgentSessionNavigationSection[],
  targetId: string,
) {
  for (const section of sections) {
    const found = findAncestors(section.children, targetId);
    if (found.length > 0 || section.children.some(({ id }) => id === targetId)) return [...found];
  }
  return [] as string[];
}

function nodeContains(
  sections: readonly AgentSessionNavigationSection[],
  nodeId: string,
  targetId: string,
) {
  for (const section of sections) {
    const node = findNode(section.children, nodeId);
    if (node?.kind === 'folder' && findNode(node.children, targetId)) return true;
  }
  return false;
}

function findNode(
  nodes: readonly AgentSessionNavigationNode[],
  targetId: string,
): AgentSessionNavigationNode | undefined {
  for (const node of nodes) {
    if (node.id === targetId) return node;
    if (node.kind === 'folder') {
      const found = findNode(node.children, targetId);
      if (found) return found;
    }
  }
  return undefined;
}

function findAncestors(
  nodes: readonly AgentSessionNavigationNode[],
  targetId: string,
  ancestors: readonly string[] = [],
): readonly string[] {
  for (const node of nodes) {
    if (node.id === targetId) return ancestors;
    if (node.kind === 'folder') {
      const found = findAncestors(node.children, targetId, [...ancestors, node.id]);
      if (found.length > 0) return found;
    }
  }
  return [];
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(
    new Date(value),
  );
}
