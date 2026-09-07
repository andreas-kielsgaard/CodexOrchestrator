import { useId, useRef, useState } from 'react';
import {
  beginWorkflowNodeDrag,
  projectWorkflowNodeDrag,
  type WorkflowNodeDragState,
  type WorkflowNodeDragPreview,
} from '../workflows/editor/workflowNodeDrag';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import '../workflows/workflow.css';
import './workflowCanvas.css';

export interface CanvasNode {
  readonly id: string;
  readonly name: string;
  readonly x: number;
  readonly y: number;
  readonly starting: boolean;
}
export interface CanvasConnection {
  readonly id: string;
  readonly name: string;
  readonly source: string;
  readonly destination: string;
}

/** Layout and gestures only. Profile editing, persistence and execution belong to the caller. */
export function WorkflowCanvas({
  nodes,
  connections,
  selection,
  canAdd,
  onSelect,
  onPlace,
  onConnect,
  onMove,
  onRemove,
  onStartingNode,
  canUndo,
  canRedo,
  onUndo,
  onRedo,
}: {
  readonly nodes: readonly CanvasNode[];
  readonly connections: readonly CanvasConnection[];
  readonly selection: WorkflowEditorSelection;
  readonly canAdd: boolean;
  onSelect(selection: WorkflowEditorSelection): void;
  onPlace(x: number, y: number, copyFrom?: string): void;
  onConnect(source: string, destination: string): void;
  onMove(id: string, x: number, y: number): void;
  onRemove(): void;
  onStartingNode(id: string): void;
  readonly canUndo?: boolean;
  readonly canRedo?: boolean;
  onUndo?(): void;
  onRedo?(): void;
}) {
  const [brush, setBrush] = useState<'select' | 'node' | 'connection' | 'copy'>('select');
  const [source, setSource] = useState<string | null>(null);
  const drag = useRef<WorkflowNodeDragState | null>(null);
  const [preview, setPreview] = useState<WorkflowNodeDragPreview | null>(null);
  const suppressed = useRef(false);
  const arrowId = useId();
  const width = Math.max(1000, ...nodes.map((node) => node.x + 320));
  const height = Math.max(620, ...nodes.map((node) => node.y + 220));
  const located = nodes.map((node) =>
    preview?.nodeId === node.id ? { ...node, x: preview.positionX, y: preview.positionY } : node,
  );
  const chooseBrush = (next: typeof brush) => {
    setBrush(next);
    setSource(null);
    onSelect({ kind: 'node', id: null });
  };
  const place = (x: number, y: number) => {
    if (brush === 'node' || (brush === 'copy' && source)) {
      onPlace(Math.max(24, x), Math.max(28, y), brush === 'copy' ? source! : undefined);
      setBrush('select');
      setSource(null);
    } else onSelect({ kind: 'node', id: null });
  };
  return (
    <section className="recipe-canvas-workspace">
      <div className="recipe-canvas-toolbar" role="toolbar" aria-label="Flow tools">
        <button type="button" disabled={!canUndo} onClick={onUndo}>
          Undo
        </button>
        <button type="button" disabled={!canRedo} onClick={onRedo}>
          Redo
        </button>
        {(['select', 'node', 'connection', 'copy'] as const).map((tool) => (
          <button
            type="button"
            key={tool}
            aria-pressed={brush === tool}
            disabled={
              (tool === 'node' && !canAdd) ||
              ((tool === 'connection' || tool === 'copy') && !nodes.length)
            }
            onClick={() => chooseBrush(tool)}
          >
            {{ select: 'Select', node: 'Add node', connection: 'Connect', copy: 'Copy node' }[tool]}
          </button>
        ))}
        <button type="button" disabled={!selection.id} onClick={onRemove}>
          Delete selected
        </button>
        <button
          type="button"
          disabled={selection.kind !== 'node' || !selection.id}
          onClick={() => selection.id && onStartingNode(selection.id)}
        >
          Set as start
        </button>
      </div>
      <p className="recipe-canvas-help">
        {brush === 'node'
          ? 'Click the canvas to place a node.'
          : brush === 'connection'
            ? source
              ? 'Choose the destination node.'
              : 'Choose the source node, then the destination.'
            : brush === 'copy'
              ? source
                ? 'Click the canvas to place the copy.'
                : 'Choose a node to copy.'
              : 'Click a node or connection to edit. Drag nodes to move them.'}
      </p>
      {!canAdd ? (
        <p className="recipe-canvas-help">Create a Capability Profile before adding nodes.</p>
      ) : null}
      <div className="recipe-canvas-scroll">
        <div
          className="workflow-canvas recipe-canvas"
          role="region"
          aria-label="Workflow canvas"
          tabIndex={0}
          style={{ width, height }}
          onClick={(event) => {
            if (event.target === event.currentTarget) {
              const rect = event.currentTarget.getBoundingClientRect();
              place(event.clientX - rect.left, event.clientY - rect.top);
            }
          }}
          onKeyDown={(event) => {
            if (
              event.target === event.currentTarget &&
              (event.key === 'Enter' || event.key === ' ')
            ) {
              event.preventDefault();
              place(50 + nodes.length * 45, 60 + nodes.length * 45);
            }
          }}
        >
          <svg className="workflow-canvas__connections" aria-label="Workflow connections">
            <defs>
              <marker id={arrowId} markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
                <path d="M0,0 L8,4 L0,8 z" />
              </marker>
            </defs>
            {connections.map((edge) => {
              const from = located.find((node) => node.id === edge.source);
              const to = located.find((node) => node.id === edge.destination);
              if (!from || !to) return null;
              const select = () => onSelect({ kind: 'connection', id: edge.id });
              return (
                <g
                  key={edge.id}
                  role="button"
                  tabIndex={0}
                  aria-label={`Edit ${edge.name}`}
                  aria-pressed={selection.kind === 'connection' && selection.id === edge.id}
                  onClick={(event) => {
                    event.stopPropagation();
                    select();
                  }}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      select();
                    }
                  }}
                >
                  <line
                    className="workflow-connection__visible"
                    x1={from.x + 210}
                    y1={from.y + 46}
                    x2={to.x}
                    y2={to.y + 46}
                    markerEnd={`url(#${arrowId})`}
                  />
                  <line
                    className="workflow-connection__hitbox"
                    x1={from.x + 210}
                    y1={from.y + 46}
                    x2={to.x}
                    y2={to.y + 46}
                  />
                  <text x={(from.x + 210 + to.x) / 2} y={(from.y + to.y) / 2 + 36}>
                    {edge.name}
                  </text>
                </g>
              );
            })}
          </svg>
          {located.map((node) => (
            <button
              type="button"
              key={node.id}
              aria-label={`Configure ${node.name}`}
              aria-pressed={selection.kind === 'node' && selection.id === node.id}
              className={`workflow-node${node.starting ? ' is-start' : ''}${selection.id === node.id ? ' is-selected' : ''}${source === node.id ? ' is-connection-source' : ''}`}
              style={{ left: node.x, top: node.y }}
              onPointerDown={(event) => {
                if (brush !== 'select' || event.button > 0) return;
                drag.current = beginWorkflowNodeDrag({
                  nodeId: node.id,
                  pointerId: event.pointerId,
                  positionX: node.x,
                  positionY: node.y,
                  clientX: event.clientX,
                  clientY: event.clientY,
                });
                event.currentTarget.setPointerCapture?.(event.pointerId);
              }}
              onPointerMove={(event) => {
                if (drag.current?.pointerId === event.pointerId)
                  setPreview(
                    projectWorkflowNodeDrag(drag.current, event.clientX, event.clientY, {
                      width,
                      height,
                    }),
                  );
              }}
              onPointerUp={(event) => {
                if (!drag.current || drag.current.pointerId !== event.pointerId) return;
                const next = projectWorkflowNodeDrag(drag.current, event.clientX, event.clientY, {
                  width,
                  height,
                });
                drag.current = null;
                setPreview(null);
                if (next.moved) {
                  suppressed.current = true;
                  onMove(node.id, next.positionX, next.positionY);
                }
                event.currentTarget.releasePointerCapture?.(event.pointerId);
              }}
              onPointerCancel={() => {
                drag.current = null;
                setPreview(null);
              }}
              onKeyDown={(event) => {
                if (
                  brush !== 'select' ||
                  !event.altKey ||
                  !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)
                )
                  return;
                event.preventDefault();
                onMove(
                  node.id,
                  Math.max(
                    24,
                    node.x +
                      (event.key === 'ArrowRight' ? 20 : event.key === 'ArrowLeft' ? -20 : 0),
                  ),
                  Math.max(
                    28,
                    node.y + (event.key === 'ArrowDown' ? 20 : event.key === 'ArrowUp' ? -20 : 0),
                  ),
                );
              }}
              onClick={(event) => {
                event.stopPropagation();
                if (suppressed.current) {
                  suppressed.current = false;
                  return;
                }
                if (brush === 'copy') {
                  setSource(node.id);
                  return;
                }
                if (brush === 'connection') {
                  if (source && source !== node.id) {
                    onConnect(source, node.id);
                    setSource(null);
                    setBrush('select');
                  } else setSource(node.id);
                  return;
                }
                onSelect({ kind: 'node', id: node.id });
              }}
            >
              <span className="workflow-node__badges">
                {node.starting ? <small>Start</small> : null}
              </span>
              <strong>{node.name}</strong>
              <span>Node profile</span>
            </button>
          ))}
          {!nodes.length ? (
            <div className="workflow-canvas__empty">
              <strong>Add your first node</strong>
              <span>Choose Add node, then click the canvas.</span>
            </div>
          ) : null}
        </div>
      </div>
    </section>
  );
}
