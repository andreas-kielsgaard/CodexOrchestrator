import type { ExtensionWorkflowDefinition } from '../../application/extensionWorkflows/contracts';
import { WorkflowGraphCanvas } from '../workflowGraph';
export type ExtensionWorkflowSelection = { kind: 'node' | 'connection'; id: string };
const groups = [
  { id: 'capture-group', name: 'Capture and classification', nodeIds: ['transcript', 'filter', 'buffer'] },
  { id: 'work-group', name: 'Work routing', nodeIds: ['router', 'workers'] },
  { id: 'response-group', name: 'Response', nodeIds: ['summary', 'digest', 'output'] },
];
const contextEdges = new Set(['buffer-digest', 'summary-digest', 'workers-digest', 'router-digest', 'progress-digest']);
const feedbackEdges = new Set(['workers-router', 'output-router']);
/** Voice supplies its content and semantics; the shared canvas owns navigation and layout. */
export function ExtensionWorkflowCanvas({ definition, selection, onSelect, statuses, highlightedIds = [] }: {
  definition: ExtensionWorkflowDefinition; selection: ExtensionWorkflowSelection;
  onSelect(selection: ExtensionWorkflowSelection): void; statuses: Record<string, string>; highlightedIds?: readonly string[];
}) {
  return <WorkflowGraphCanvas autoLayout label={`${definition.name} workflow`} nodes={definition.nodes}
    connections={definition.connections.map(edge => ({ ...edge, role: feedbackEdges.has(edge.id) ? 'feedback' : contextEdges.has(edge.id) ? 'context' : 'flow' }))}
    selection={selection} highlightedIds={highlightedIds} groups={groups.filter(group => group.nodeIds.every(id => definition.nodes.some(n => n.id === id)))}
    nodeLabel={node => `Inspect ${node.name}`} onNodeActivate={node => onSelect({ kind: 'node', id: node.id })}
    onConnectionActivate={ids => onSelect({ kind: 'connection', id: ids[0] })}
    renderNode={node => <><small>{definition.nodes.find(n => n.id === node.id)?.kind.replaceAll('_', ' ')}</small><strong>{node.name}</strong><span>{statuses[node.id] ?? 'Not started'}</span></>} />;
}
