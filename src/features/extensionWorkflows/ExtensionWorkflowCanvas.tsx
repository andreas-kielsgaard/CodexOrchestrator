import { useRef, useState } from 'react';
import type { Viewport } from '@xyflow/react';
import type { ExtensionWorkflowDefinition } from '../../application/extensionWorkflows/contracts';
import { WorkflowGraphCanvas } from '../workflowGraph';
export type ExtensionWorkflowSelection = { kind: 'node' | 'connection'; id: string } | null;
const contextEdges = new Set([
  'buffer-digest',
  'summary-digest',
  'workers-digest',
  'router-digest',
  'progress-digest',
]);
const feedbackEdges = new Set(['workers-router', 'output-router']);

/** Voice supplies its semantics; React Flow and ELK own navigation and layout. */
export function ExtensionWorkflowCanvas({
  definition,
  selection,
  onSelect,
  statuses,
  highlightedIds = [],
  traceMode = false,
}: {
  definition: ExtensionWorkflowDefinition;
  selection: ExtensionWorkflowSelection;
  onSelect(selection: ExtensionWorkflowSelection): void;
  statuses: Record<string, string>;
  highlightedIds?: readonly string[];
  traceMode?: boolean;
}) {
  const [focused, setFocused] = useState(false);
  const [context, setContext] = useState(true);
  const viewports = useRef<Record<string, Viewport>>({});
  const overviewSelection = useRef<ExtensionWorkflowSelection>(null);
  const selectedEdge = definition.connections.find(
    (edge) => selection?.kind === 'connection' && edge.id === selection.id,
  );
  const focusId = focused
    ? selection?.kind === 'node'
      ? selection.id
      : selectedEdge?.source
    : undefined;
  const focusNode = definition.nodes.find((node) => node.id === focusId);
  const connections = definition.connections
    .filter((edge) =>
      focusId
        ? edge.source === focusId || edge.destination === focusId
        : context ||
          (!contextEdges.has(edge.id) && !feedbackEdges.has(edge.id)) ||
          edge.id === selectedEdge?.id ||
          highlightedIds.includes(edge.id),
    )
    .map((edge) => ({
      ...edge,
      role: feedbackEdges.has(edge.id)
        ? ('feedback' as const)
        : contextEdges.has(edge.id)
          ? ('context' as const)
          : ('flow' as const),
    }));
  const visibleIds = new Set(connections.flatMap((edge) => [edge.source, edge.destination]));
  const nodes = focusId
    ? definition.nodes.filter((node) => node.id === focusId || visibleIds.has(node.id))
    : definition.nodes;
  const scope = focusId ? `stage:${focusId}` : context ? 'overview:all' : 'overview:primary';
  return (
    <section className="voice-map" aria-label="Voice flow map">
      <div className="voice-map-navigation">
        {focusNode ? (
          <>
            <button
              onClick={() => {
                setFocused(false);
                onSelect(overviewSelection.current);
              }}
            >
              ← All stages
            </button>
            <strong>/ {focusNode.name}</strong>
          </>
        ) : (
          <strong>All stages</strong>
        )}
        <select
          aria-label="Inspect stage"
          value={selection?.kind === 'node' ? selection.id : ''}
          onChange={(event) => {
            onSelect(event.target.value ? { kind: 'node', id: event.target.value } : null);
            if (!event.target.value) setFocused(false);
          }}
        >
          <option value="">Workflow overview</option>
          {definition.nodes.map((node) => (
            <option key={node.id} value={node.id}>
              {node.name}
            </option>
          ))}
        </select>
        {!focused && selection && (
          <button
            onClick={() => {
              overviewSelection.current = selection;
              setFocused(true);
            }}
          >
            Focus{' '}
            {definition.nodes.find(
              (n) => n.id === (selection.kind === 'node' ? selection.id : selectedEdge?.source),
            )?.name ?? 'stage'}
          </button>
        )}
        {!focused && (
          <label className="voice-check">
            <input
              type="checkbox"
              checked={context}
              onChange={(e) => setContext(e.target.checked)}
            />
            Context and feedback links
          </label>
        )}
      </div>
      <WorkflowGraphCanvas
        key={scope}
        autoLayout={focused}
        compactNavigation
        initialViewport={viewports.current[scope]}
        onViewportChange={(viewport) => {
          viewports.current[scope] = viewport;
        }}
        label={`${definition.name} workflow`}
        nodes={nodes}
        connections={connections}
        selection={traceMode ? null : selection}
        highlightedIds={highlightedIds}
        nodeLabel={(node) => `Inspect ${node.name}`}
        onNodeActivate={(node) => onSelect({ kind: 'node', id: node.id })}
        onConnectionActivate={(ids) => onSelect({ kind: 'connection', id: ids[0] })}
        renderNode={(node) => (
          <>
            <small>
              {definition.nodes.find((n) => n.id === node.id)?.kind.replaceAll('_', ' ')}
            </small>
            <strong>{node.name}</strong>
            <span>{statuses[node.id] ?? 'Not started'}</span>
          </>
        )}
      />
      <p className="voice-map-caption">
        {traceMode && 'Highlighted connections are the recorded route for the selected event. '}
        {focusNode
          ? 'Showing this stage and its incoming and outgoing connections.'
          : context
            ? 'All control, context and feedback connections.'
            : 'Primary routes shown. Context and feedback links are available above and in each stage’s inspector.'}{' '}
        Dashed routes carry context; dotted routes are disabled. Parallel branches can run
        independently.
      </p>
    </section>
  );
}
