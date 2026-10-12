import { useState } from 'react';
import { LogIn } from 'lucide-react';
import {
  WorkflowGraphCanvas,
  snapWorkflowPoint,
  type WorkflowGraphConnection,
  type WorkflowGraphNode,
  type GraphLayout,
} from '../workflowGraph';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';
import './workflowCanvas.css';
export type CanvasNode = WorkflowGraphNode & { readonly entry: boolean };
export type CanvasConnection = WorkflowGraphConnection;
/** Draft edits remain owned by authoring. */
export function WorkflowCanvas({
  nodes,
  connections,
  selection,
  canAdd,
  onSelect,
  onPlace,
  onConnect,
  onMove,
  onOrganize,
  onRemove,
  onEntryNode,
  onConnectionGroup,
  canUndo,
  canRedo,
  onUndo,
  onRedo,
}: {
  nodes: readonly CanvasNode[];
  connections: readonly CanvasConnection[];
  selection: WorkflowEditorSelection;
  canAdd: boolean;
  onSelect(selection: WorkflowEditorSelection): void;
  onPlace(x: number, y: number, copyFrom?: string): void;
  onConnect(source: string, destination: string): void;
  onMove(id: string, x: number, y: number): void;
  onOrganize?(positions: GraphLayout['positions']): void;
  onRemove(): void;
  onEntryNode(id: string): void;
  onConnectionGroup(ids: readonly string[]): void;
  canUndo?: boolean;
  canRedo?: boolean;
  onUndo?(): void;
  onRedo?(): void;
}) {
  const [brush, setBrush] = useState<'select' | 'node' | 'connection' | 'copy'>('select');
  const [source, setSource] = useState<string | null>(null);
  const choosing = (next: typeof brush) => {
    setBrush(next);
    setSource(null);
    onSelect({ kind: 'node', id: null });
  };
  return (
    <section className="recipe-canvas-workspace">
      <div className="recipe-canvas-toolbar" role="toolbar" aria-label="Flow tools">
        <button disabled={!canUndo} onClick={onUndo}>
          Undo
        </button>
        <button disabled={!canRedo} onClick={onRedo}>
          Redo
        </button>
        {(['select', 'node', 'connection', 'copy'] as const).map((tool) => (
          <button
            key={tool}
            aria-pressed={brush === tool}
            disabled={tool === 'node' ? !canAdd : tool !== 'select' && !nodes.length}
            onClick={() => choosing(tool)}
          >
            {{ select: 'Select', node: 'Add node', connection: 'Connect', copy: 'Copy node' }[tool]}
          </button>
        ))}
        <button disabled={!selection.id} onClick={onRemove}>
          Delete selected
        </button>
      </div>
      <p className="recipe-canvas-help">
        {brush === 'node'
          ? 'Click the canvas to place a node.'
          : brush === 'copy'
            ? source
              ? 'Click the canvas to place the copy.'
              : 'Choose a node to copy.'
            : brush === 'connection'
              ? source
                ? 'Choose the destination node.'
                : 'Choose the source node, then the destination.'
              : 'Select a node or connection. Drag nodes to move them; Organize arranges the draft.'}
      </p>
      {!canAdd && (
        <p className="recipe-canvas-help">Create a Capability Profile before adding nodes.</p>
      )}
      <WorkflowGraphCanvas
        label="Workflow canvas"
        nodes={nodes}
        connections={connections}
        selection={selection}
        nodeLabel={(node) => `Configure ${node.name}`}
        connectionLabel={(edge) => `Edit ${edge.name}`}
        onConnectionActivate={onConnectionGroup}
        onOrganize={onOrganize}
        onMove={
          brush === 'select'
            ? (id, x, y) => {
                const p = snapWorkflowPoint(x, y);
                onMove(id, p.x, p.y);
              }
            : undefined
        }
        placing={
          brush === 'node' ? 'New node' : brush === 'copy' && source ? 'Node copy' : undefined
        }
        onPlace={(x, y, snap) => {
          if (brush === 'node' || (brush === 'copy' && source)) {
            const point = snapWorkflowPoint(x, y, undefined, snap);
            onPlace(point.x, point.y, brush === 'copy' ? source! : undefined);
            setBrush('select');
            setSource(null);
          } else onSelect({ kind: 'node', id: null });
        }}
        onNodeActivate={(node) => {
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
        renderNode={(node) => (
          <>
            <span className="workflow-node__badges">{node.entry && <small>Entry</small>}</span>
            <strong>{node.name}</strong>
            <span>Node profile</span>
          </>
        )}
        renderActions={(node) => (
          <button
            className="workflow-node__entry-toggle"
            aria-label={
              node.entry
                ? `Remove ${node.name} as an entry node`
                : `Make ${node.name} an entry node`
            }
            aria-pressed={node.entry}
            onClick={() => onEntryNode(node.id)}
          >
            <LogIn size={14} aria-hidden="true" />
          </button>
        )}
      />
    </section>
  );
}
