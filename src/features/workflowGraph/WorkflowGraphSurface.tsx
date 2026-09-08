import { useId, type ButtonHTMLAttributes, type HTMLAttributes, type ReactNode } from 'react';
import {
  WORKFLOW_GRAPH_CONNECTION_Y,
  WORKFLOW_GRAPH_NODE_WIDTH,
  workflowGraphNodeById,
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
  ...canvasProps
}: {
  readonly width: number;
  readonly height: number;
  readonly scrollClassName?: string;
  readonly children: ReactNode;
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={scrollClassName ?? 'workflow-graph-scroll'}>
      <div
        {...canvasProps}
        className={`workflow-canvas${className ? ` ${className}` : ''}`}
        style={{ ...canvasProps.style, width, height }}
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
  onActivate?(connection: WorkflowGraphConnection): void;
}) {
  const arrowId = useId();
  return (
    <svg className="workflow-canvas__connections" aria-label="Workflow connections">
      <defs>
        <marker id={arrowId} markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
          <path d="M0,0 L8,4 L0,8 z" />
        </marker>
      </defs>
      {connections.map((connection) => {
        const from = workflowGraphNodeById(nodes, connection.source);
        const to = workflowGraphNodeById(nodes, connection.destination);
        if (!from || !to) return null;
        const activate = () => onActivate?.(connection);
        const interactive = Boolean(onActivate);
        return (
          <g
            key={connection.id}
            role={interactive ? 'button' : undefined}
            tabIndex={interactive ? 0 : undefined}
            aria-label={interactive ? ariaLabelForConnection(connection) : undefined}
            aria-pressed={interactive ? selectedId === connection.id : undefined}
            className={`${selectedId === connection.id ? 'is-selected' : ''}${highlightedIds.includes(connection.id) ? ' is-highlighted' : ''}`}
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
              x1={from.x + WORKFLOW_GRAPH_NODE_WIDTH}
              y1={from.y + WORKFLOW_GRAPH_CONNECTION_Y}
              x2={to.x}
              y2={to.y + WORKFLOW_GRAPH_CONNECTION_Y}
              markerEnd={`url(#${arrowId})`}
            />
            {interactive ? (
              <line
                className="workflow-connection__hitbox"
                x1={from.x + WORKFLOW_GRAPH_NODE_WIDTH}
                y1={from.y + WORKFLOW_GRAPH_CONNECTION_Y}
                x2={to.x}
                y2={to.y + WORKFLOW_GRAPH_CONNECTION_Y}
              />
            ) : null}
            <text x={(from.x + WORKFLOW_GRAPH_NODE_WIDTH + to.x) / 2} y={(from.y + to.y) / 2 + 36}>
              {labelForConnection(connection)}
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
  ...buttonProps
}: {
  readonly node: WorkflowGraphNode;
  readonly selected?: boolean;
  readonly children: ReactNode;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'>) {
  return (
    <button
      type="button"
      {...buttonProps}
      aria-pressed={buttonProps['aria-pressed'] ?? selected}
      className={`workflow-node${node.starting ? ' is-start' : ''}${selected ? ' is-selected' : ''}${className ? ` ${className}` : ''}`}
      style={{ ...buttonProps.style, left: node.x, top: node.y }}
    >
      {children}
    </button>
  );
}

export function WorkflowGraphEmpty({ children }: { readonly children: ReactNode }) {
  return <div className="workflow-canvas__empty">{children}</div>;
}
