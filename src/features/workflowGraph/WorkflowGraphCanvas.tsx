import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  MiniMap,
  useReactFlow,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type Viewport,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import {
  organizeWorkflow,
  connectionPort,
  projectWorkflowGroups,
  type GraphLayout,
  type WorkflowVisualGroup,
} from './workflowLayout';
import {
  WORKFLOW_GRAPH_NODE_WIDTH,
  WORKFLOW_GRAPH_NODE_HEIGHT,
  type WorkflowGraphNode,
  type WorkflowGraphConnection,
} from './workflowGraphModel';
import './workflowGraphCanvas.css';
import {
  WorkflowCard,
  WorkflowConnection,
  type CardData,
  type ConnectionData,
} from './WorkflowGraphElements';

type Selection = { kind: string; id: string | null } | null | undefined;
interface CanvasProps {
  nodes: readonly WorkflowGraphNode[];
  connections: readonly WorkflowGraphConnection[];
  selection?: Selection;
  label: string;
  renderNode(node: WorkflowGraphNode): ReactNode;
  nodeLabel(node: WorkflowGraphNode): string;
  renderActions?(node: WorkflowGraphNode): ReactNode;
  onNodeActivate(node: WorkflowGraphNode): void;
  onConnectionActivate(ids: readonly string[]): void;
  onMove?(id: string, x: number, y: number): void;
  onOrganize?(positions: GraphLayout['positions']): void;
  onPlace?(x: number, y: number, snap: boolean): void;
  placing?: string;
  autoLayout?: boolean;
  highlightedIds?: readonly string[];
  labelForConnection?(edge: WorkflowGraphConnection): string;
  connectionLabel?(edge: WorkflowGraphConnection): string;
  groups?: readonly WorkflowVisualGroup[];
  compactNavigation?: boolean;
  initialViewport?: Viewport;
  onViewportChange?(viewport: Viewport): void;
}
const nodeTypes = { orchid: WorkflowCard };
const edgeTypes = { orchid: WorkflowConnection };

export function WorkflowGraphCanvas(props: CanvasProps) {
  return (
    <ReactFlowProvider>
      <Canvas {...props} />
    </ReactFlowProvider>
  );
}
function Canvas(props: CanvasProps) {
  const [collapsed, setCollapsed] = useState<Set<string>>(
    () => new Set(props.groups?.map((group) => group.id)),
  );
  const [allLabels, setAllLabels] = useState(false);
  const [search, setSearch] = useState('');
  const [layout, setLayout] = useState<GraphLayout | null>(null);
  const [layoutError, setLayoutError] = useState('');
  const [organizing, setOrganizing] = useState(false);
  const [preview, setPreview] = useState<{ x: number; y: number }>();
  const pendingViewport = useRef<Viewport | 'fit' | null>(null);
  const flow = useReactFlow();
  const projection = projectWorkflowGroups(
    props.nodes,
    props.connections,
    props.groups ?? [],
    collapsed,
  );
  const currentInputs = JSON.stringify({
    nodes: projection.nodes,
    connections: projection.connections,
  });
  const latest = useRef({ props, currentInputs });
  latest.current = { props, currentInputs };
  const topology = JSON.stringify({
    nodes: projection.nodes.map((n) => n.id),
    edges: projection.connections.map((e) => [e.id, e.source, e.destination, e.role]),
  });
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<CardData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge<ConnectionData>>([]);
  const organize = async (persist: boolean, fit = true) => {
    setOrganizing(true);
    setLayoutError('');
    try {
      const result = await organizeWorkflow(projection.nodes, projection.connections);
      if (latest.current.currentInputs !== currentInputs) return;
      pendingViewport.current = fit ? 'fit' : null;
      setLayout(result);
      if (persist) latest.current.props.onOrganize?.(result.positions);
    } catch (error) {
      setLayoutError(String(error));
    } finally {
      setOrganizing(false);
    }
  };
  useEffect(() => {
    let active = true;
    if (props.autoLayout || collapsed.size) {
      void organizeWorkflow(projection.nodes, projection.connections)
        .then((result) => {
          if (active) {
            pendingViewport.current = props.initialViewport ?? 'fit';
            setLayout(result);
          }
        })
        .catch((error) => {
          if (active) setLayoutError(String(error));
        });
    } else setLayout(null);
    return () => {
      active = false;
    };
    // Layout responds to topology, never status polling or selection.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [topology, props.autoLayout]);
  const selectedId =
    props.selection?.kind === 'node'
      ? (props.groups?.find(
          (group) => collapsed.has(group.id) && group.nodeIds.includes(props.selection?.id ?? ''),
        )?.id ?? props.selection.id)
      : props.selection?.id;
  const connected = new Set(
    projection.connections
      .filter(
        (e) =>
          e.source === selectedId ||
          e.destination === selectedId ||
          e.id === selectedId ||
          props.highlightedIds?.includes(e.id),
      )
      .flatMap((e) => [e.source, e.destination]),
  );
  const displayNodes = projection.nodes.map((node) => {
    const group = props.groups?.find((g) => g.id === node.id && collapsed.has(g.id));
    const position = layout?.positions[node.id] ?? { x: node.x, y: node.y };
    return {
      id: node.id,
      type: 'orchid',
      position,
      width: WORKFLOW_GRAPH_NODE_WIDTH,
      height: WORKFLOW_GRAPH_NODE_HEIGHT,
      dragHandle: '.workflow-flow-card__drag',
      selected: props.selection?.kind === 'node' && node.id === selectedId,
      draggable: Boolean(props.onMove) && !props.placing && !group,
      deletable: false,
      connectable: false,
      style: {
        opacity:
          (selectedId || props.highlightedIds?.length) &&
          !connected.has(node.id) &&
          node.id !== selectedId
            ? 0.5
            : 1,
      },
      data: {
        content: group ? (
          <>
            <strong>{group.name}</strong>
            <span>{group.nodeIds.length} nodes · expand</span>
          </>
        ) : (
          props.renderNode(node)
        ),
        actions: group ? undefined : props.renderActions?.(node),
        label: group ? `Expand ${group.name}` : props.nodeLabel(node),
        activate: () =>
          group
            ? setCollapsed((current) => new Set([...current].filter((id) => id !== group.id)))
            : props.onNodeActivate(node),
        move: props.onMove
          ? (x: number, y: number) => {
              setLayout(null);
              props.onMove?.(node.id, x, y);
            }
          : undefined,
        ports: projection.connections.flatMap((edge) => [
          ...(edge.source === node.id
            ? [
                {
                  id: connectionPort(edge.id, 'out'),
                  direction: 'out' as const,
                  feedback: edge.role === 'feedback',
                  position: layout?.ports[connectionPort(edge.id, 'out')],
                },
              ]
            : []),
          ...(edge.destination === node.id
            ? [
                {
                  id: connectionPort(edge.id, 'in'),
                  direction: 'in' as const,
                  feedback: edge.role === 'feedback',
                  position: layout?.ports[connectionPort(edge.id, 'in')],
                },
              ]
            : []),
        ]),
      },
    };
  });
  const displayEdges = projection.connections.map((edge) => {
    const highlighted = props.highlightedIds?.includes(edge.id);
    const focused =
      edge.source === selectedId ||
      edge.destination === selectedId ||
      edge.id === selectedId ||
      highlighted;
    return {
      id: edge.id,
      source: edge.source,
      target: edge.destination,
      sourceHandle: connectionPort(edge.id, 'out'),
      targetHandle: connectionPort(edge.id, 'in'),
      type: 'orchid',
      selected: edge.id === selectedId,
      deletable: false,
      markerEnd: {
        type: 'arrowclosed' as const,
        color: focused ? 'var(--accent)' : 'var(--graph-edge)',
      },
      style: {
        stroke: focused ? 'var(--accent)' : 'var(--graph-edge)',
        strokeWidth: focused ? 3 : 1.5,
        strokeDasharray:
          edge.enabled === false ? '2 6' : edge.role === 'context' ? '5 5' : undefined,
        opacity:
          (selectedId || props.highlightedIds?.length) && !focused
            ? 0.2
            : edge.role === 'context'
              ? 0.55
              : 1,
      },
      data: {
        route: layout?.routes[edge.id],
        label: props.labelForConnection?.(edge) ?? edge.name,
        title: props.connectionLabel?.(edge) ?? `Open ${edge.name}`,
        showLabel: allLabels || Boolean(focused),
        activate: () => props.onConnectionActivate([edge.id]),
      },
    };
  });
  // Node content includes live status. Updating it must not reset the viewport.
  useEffect(
    () => {
      setNodes(displayNodes);
      setEdges(displayEdges);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      props.nodes,
      props.connections,
      props.selection,
      props.renderNode,
      props.nodeLabel,
      props.renderActions,
      props.onNodeActivate,
      props.onConnectionActivate,
      props.onMove,
      props.highlightedIds,
      props.labelForConnection,
      props.connectionLabel,
      layout,
      collapsed,
      allLabels,
    ],
  );
  // Fit only after React Flow has received the arranged positions, not the source coordinates.
  useEffect(() => {
    if (
      !pendingViewport.current ||
      !layout ||
      !nodes.length ||
      !nodes.every(
        (node) =>
          node.position.x === layout.positions[node.id]?.x &&
          node.position.y === layout.positions[node.id]?.y,
      )
    )
      return;
    const viewport = pendingViewport.current;
    pendingViewport.current = null;
    if (viewport === 'fit')
      void flow.fitView({ padding: 0.15, ...(props.compactNavigation ? { maxZoom: 1 } : {}) });
    else void flow.setViewport(viewport);
  }, [nodes, layout, flow, props.compactNavigation]);
  const matching = useMemo(
    () => props.nodes.filter((n) => search && n.name.toLowerCase().includes(search.toLowerCase())),
    [search, props.nodes],
  );
  const selectNode = (id: string) => {
    setCollapsed(
      (current) =>
        new Set(
          [...current].filter(
            (group) => !props.groups?.find((g) => g.id === group)?.nodeIds.includes(id),
          ),
        ),
    );
    props.onNodeActivate(props.nodes.find((n) => n.id === id)!);
    requestAnimationFrame(
      () => void flow.fitView({ nodes: [{ id }], maxZoom: 1, padding: 0.5, duration: 200 }),
    );
  };
  return (
    <section
      className="workflow-flow"
      aria-label={props.label}
      role="region"
      tabIndex={0}
      onClick={(event) => {
        if (event.target === event.currentTarget && props.onPlace) {
          const point = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
          props.onPlace(point.x, point.y, !event.altKey);
        }
      }}
      onKeyDown={(event) => {
        if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) {
          event.preventDefault();
          props.onPlace?.(50 + props.nodes.length * 45, 60 + props.nodes.length * 45, true);
        }
      }}
    >
      {!props.compactNavigation && (
        <div className="workflow-flow-toolbar" role="toolbar" aria-label="Graph navigation">
          {!props.compactNavigation && (
            <button onClick={() => void organize(Boolean(props.onOrganize))} disabled={organizing}>
              {organizing ? 'Organizing…' : 'Organize'}
            </button>
          )}
          <button onClick={() => void flow.fitView({ padding: 0.15, duration: 200 })}>
            Fit graph
          </button>
          <button
            disabled={!selectedId}
            onClick={() =>
              void flow.fitView({
                nodes: nodes.filter((n) => n.id === selectedId || connected.has(n.id)),
                padding: 0.2,
                maxZoom: 1,
                duration: 200,
              })
            }
          >
            Fit selection
          </button>
          {!!props.groups?.length && (
            <button
              onClick={() =>
                setCollapsed((current) =>
                  current.size ? new Set() : new Set(props.groups?.map((group) => group.id)),
                )
              }
            >
              {collapsed.size ? 'All nodes' : 'Overview'}
            </button>
          )}
          {!props.compactNavigation && (
            <label>
              <input
                type="checkbox"
                checked={allLabels}
                onChange={(e) => setAllLabels(e.target.checked)}
              />{' '}
              All labels
            </label>
          )}
          {!props.compactNavigation && (
            <input
              type="search"
              aria-label="Find node"
              placeholder="Find node"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          )}
          {(props.groups ?? []).map((group) => (
            <button
              key={group.id}
              aria-pressed={collapsed.has(group.id)}
              onClick={() =>
                setCollapsed((current) => {
                  const next = new Set(current);
                  if (next.has(group.id)) next.delete(group.id);
                  else next.add(group.id);
                  return next;
                })
              }
            >
              {collapsed.has(group.id) ? 'Expand' : 'Collapse'} {group.name}
            </button>
          ))}
        </div>
      )}
      {matching.length > 0 && (
        <div className="workflow-flow-search">
          {matching.map((n) => (
            <button key={n.id} onClick={() => selectNode(n.id)}>
              {n.name}
            </button>
          ))}
        </div>
      )}
      {layoutError && <p role="alert">Layout: {layoutError}</p>}
      <div className="workflow-flow-viewport">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          fitView={!props.initialViewport && !props.autoLayout && !collapsed.size}
          fitViewOptions={props.compactNavigation ? { padding: 0.08, maxZoom: 1 } : undefined}
          defaultViewport={props.initialViewport}
          onMoveEnd={(_, viewport) => props.onViewportChange?.(viewport)}
          minZoom={0.2}
          maxZoom={1.8}
          nodesConnectable={false}
          nodesFocusable
          edgesFocusable
          deleteKeyCode={null}
          panOnScroll
          zoomOnDoubleClick={false}
          onNodeDragStop={(_, node) => {
            setLayout(null);
            props.onMove?.(node.id, node.position.x, node.position.y);
          }}
          onEdgeClick={(_, edge) => props.onConnectionActivate([edge.id])}
          onPaneClick={(event) => {
            const point = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
            props.onPlace?.(point.x, point.y, !event.altKey);
          }}
          onPaneMouseMove={(event) => {
            if (props.placing)
              setPreview(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
          }}
        >
          <Background color="var(--graph-grid)" gap={20} />
          <Controls showInteractive={false} />
          <MiniMap
            pannable
            zoomable
            nodeColor="var(--border-strong)"
            style={{ width: 130, height: 85 }}
            maskColor="rgba(0,0,0,.3)"
          />
        </ReactFlow>
        {!nodes.length && <div className="workflow-flow-empty">Add your first node</div>}
        {props.placing && preview && (
          <span className="workflow-flow-placement">
            {props.placing} · {Math.round(preview.x)}, {Math.round(preview.y)}
          </span>
        )}
      </div>
      <details className="workflow-flow-connections">
        <summary>Connections · {props.connections.length}</summary>
        <ul>
          {props.connections.map((edge) => (
            <li key={edge.id}>
              <button
                aria-label={`${props.connectionLabel?.(edge) ?? `Open ${edge.name}`} in connection list`}
                aria-pressed={edge.id === selectedId}
                onClick={() => props.onConnectionActivate([edge.id])}
              >
                {props.nodes.find((n) => n.id === edge.source)?.name ?? edge.source} →{' '}
                {props.nodes.find((n) => n.id === edge.destination)?.name ?? edge.destination}:{' '}
                {edge.name}
                {edge.enabled === false ? ' · disabled' : ''}
              </button>
              <small>{edge.role ?? 'flow'}</small>
            </li>
          ))}
        </ul>
      </details>
    </section>
  );
}
