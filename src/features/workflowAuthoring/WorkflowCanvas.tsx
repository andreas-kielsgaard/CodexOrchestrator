import { useRef, useState } from 'react';
import {
  WorkflowGraphConnections,
  WorkflowGraphEmpty,
  WorkflowGraphNodeCard,
  WorkflowGraphSurface,
  workflowGraphBounds,
  type WorkflowGraphConnection,
  type WorkflowGraphNode,
} from '../workflowGraph';
import {
  beginWorkflowNodeDrag,
  projectWorkflowNodeDrag,
  type WorkflowNodeDragState,
  type WorkflowNodeDragPreview,
} from '../workflows/editor/workflowNodeDrag';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import './workflowCanvas.css';

export type CanvasNode = WorkflowGraphNode & { readonly starting: boolean };
export type CanvasConnection = WorkflowGraphConnection;

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
  const { width, height } = workflowGraphBounds(nodes);
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
      <WorkflowGraphSurface
        width={width}
        height={height}
        scrollClassName="recipe-canvas-scroll"
        className="recipe-canvas"
        role="region"
        aria-label="Workflow canvas"
        tabIndex={0}
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
        <WorkflowGraphConnections
          nodes={located}
          connections={connections}
          selectedId={selection.kind === 'connection' ? selection.id : null}
          ariaLabelForConnection={(connection) => `Edit ${connection.name}`}
          onActivate={(connection) => onSelect({ kind: 'connection', id: connection.id })}
        />
        {located.map((node) => (
          <WorkflowGraphNodeCard
            key={node.id}
            node={node}
            aria-label={`Configure ${node.name}`}
            selected={selection.kind === 'node' && selection.id === node.id}
            className={source === node.id ? 'is-connection-source' : undefined}
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
                  node.x + (event.key === 'ArrowRight' ? 20 : event.key === 'ArrowLeft' ? -20 : 0),
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
          </WorkflowGraphNodeCard>
        ))}
        {!nodes.length ? (
          <WorkflowGraphEmpty>
            <strong>Add your first node</strong>
            <span>Choose Add node, then click the canvas.</span>
          </WorkflowGraphEmpty>
        ) : null}
      </WorkflowGraphSurface>
    </section>
  );
}
