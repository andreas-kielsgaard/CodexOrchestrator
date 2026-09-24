import {
  useId,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type ReactNode,
  type Ref,
} from 'react';
import {
  WORKFLOW_GRAPH_CONNECTION_Y,
  WORKFLOW_GRAPH_NODE_HEIGHT,
  WORKFLOW_GRAPH_NODE_WIDTH,
  workflowGraphNodeById,
  groupWorkflowGraphConnections,
  type WorkflowGraphConnection,
  type WorkflowGraphNode,
} from './workflowGraphModel';
import './workflowGraph.css';

export function WorkflowGraphSurface({
  width,
  height,
  scrollClassName,
  className,
  children,
  canvasRef,
  ...canvasProps
}: {
  readonly width: number;
  readonly height: number;
  readonly scrollClassName?: string;
  readonly children: ReactNode;
  readonly canvasRef?: Ref<HTMLDivElement>;
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={scrollClassName ?? 'workflow-graph-scroll'}>
      <div
        ref={canvasRef}
        {...canvasProps}
        className={`workflow-canvas${className ? ` ${className}` : ''}`}
        style={{ ...canvasProps.style, width, height, minWidth: '100%', minHeight: '100%' }}
      >
        {children}
      </div>
    </div>
  );
}

export function WorkflowGraphConnections({
  nodes,
  connections,
  selectedId,
  highlightedIds = [],
  labelForConnection = (connection) => connection.name,
  ariaLabelForConnection = (connection) => `Open ${connection.name}`,
  onActivate,
}: {
  readonly nodes: readonly WorkflowGraphNode[];
  readonly connections: readonly WorkflowGraphConnection[];
  readonly selectedId?: string | null;
  readonly highlightedIds?: readonly string[];
  labelForConnection?(connection: WorkflowGraphConnection): string;
  ariaLabelForConnection?(connection: WorkflowGraphConnection): string;
  onActivate?(connectionIds: readonly string[]): void;
}) {
  const arrowId = useId();
  const groups = groupWorkflowGraphConnections(connections);
  return (
    <svg className="workflow-canvas__connections" aria-label="Workflow connections">
      <defs>
        <marker
          id={arrowId}
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="4"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L8,4 L0,8 z" />
        </marker>
      </defs>
      {groups.map((group) => {
        const from = workflowGraphNodeById(nodes, group.firstNode);
        const to = workflowGraphNodeById(nodes, group.secondNode);
        if (!from || !to) return null;
        const activate = () => onActivate?.(group.connections.map((connection) => connection.id));
        const interactive = Boolean(onActivate);
        const selected = group.connections.some((connection) => connection.id === selectedId);
        const highlighted = group.connections.some((connection) =>
          highlightedIds.includes(connection.id),
        );
        const label =
          group.connections.length === 1
            ? labelForConnection(group.connections[0])
            : `${group.connections.length} connections`;
        const firstIsLeft = from.x <= to.x;
        const x1 = firstIsLeft ? from.x + WORKFLOW_GRAPH_NODE_WIDTH : from.x;
        const x2 = firstIsLeft ? to.x : to.x + WORKFLOW_GRAPH_NODE_WIDTH;
        return (
          <g
            key={group.id}
            role={interactive ? 'button' : undefined}
            tabIndex={interactive ? 0 : undefined}
            aria-label={
              interactive
                ? group.connections.length === 1
                  ? ariaLabelForConnection(group.connections[0])
                  : `Open ${group.connections.length} connections`
                : undefined
            }
            aria-pressed={interactive ? selected : undefined}
            className={`${selected ? 'is-selected' : ''}${highlighted ? ' is-highlighted' : ''}`}
            onClick={
              interactive
                ? (event) => {
                    event.stopPropagation();
                    activate();
                  }
                : undefined
            }
            onKeyDown={
              interactive
                ? (event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      activate();
                    }
                  }
                : undefined
            }
          >
            <line
              className="workflow-connection__visible"
              x1={x1}
              y1={from.y + WORKFLOW_GRAPH_CONNECTION_Y}
              x2={x2}
              y2={to.y + WORKFLOW_GRAPH_CONNECTION_Y}
              markerStart={group.secondToFirst ? `url(#${arrowId})` : undefined}
              markerEnd={group.firstToSecond ? `url(#${arrowId})` : undefined}
            />
            {interactive ? (
              <line
                className="workflow-connection__hitbox"
                x1={x1}
                y1={from.y + WORKFLOW_GRAPH_CONNECTION_Y}
                x2={x2}
                y2={to.y + WORKFLOW_GRAPH_CONNECTION_Y}
              />
            ) : null}
            <text x={(x1 + x2) / 2} y={(from.y + to.y) / 2 + 36}>
              {label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

export function WorkflowGraphNodeCard({
  node,
  selected,
  className,
  children,
  actions,
  ...buttonProps
}: {
  readonly node: WorkflowGraphNode;
  readonly selected?: boolean;
  readonly children: ReactNode;
  readonly actions?: ReactNode;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'>) {
  return (
    <div
      className={`workflow-node${node.entry ? ' is-entry' : ''}${selected ? ' is-selected' : ''}${className ? ` ${className}` : ''}`}
      style={{
        left: node.x,
        top: node.y,
        width: WORKFLOW_GRAPH_NODE_WIDTH,
        height: WORKFLOW_GRAPH_NODE_HEIGHT,
      }}
    >
      <button
        type="button"
        {...buttonProps}
        aria-pressed={buttonProps['aria-pressed'] ?? selected}
        className="workflow-node__body"
        style={buttonProps.style}
      >
        {children}
      </button>
      {actions ? <div className="workflow-node__actions">{actions}</div> : null}
    </div>
  );
}

export function WorkflowGraphEmpty({ children }: { readonly children: ReactNode }) {
  return <div className="workflow-canvas__empty">{children}</div>;
}
