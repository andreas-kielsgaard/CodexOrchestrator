import { Plus, Trash2 } from 'lucide-react';
import type {
  CapabilityProfileDto,
  RuntimeSelectionsDto,
} from '../../application/executionConfiguration';
import type {
  WorkflowAuthoringConnectionDto,
  WorkflowAuthoringNodeDto,
  WorkflowRecipeDraftDto,
} from '../../application/workflowAuthoring';
import { defaultsWithinCapabilities } from './workflowAuthoringPresentation';
import type { WorkflowEditorSelection } from './workflowAuthoringTypes';

export function WorkflowOutline({
  draft,
  selection,
  defaultProfile,
  runtimeLockedSelections,
  onSelect,
  onChange,
}: {
  readonly draft: WorkflowRecipeDraftDto;
  readonly selection: WorkflowEditorSelection;
  readonly defaultProfile?: CapabilityProfileDto;
  readonly runtimeLockedSelections?: RuntimeSelectionsDto;
  readonly onSelect: (selection: WorkflowEditorSelection) => void;
  readonly onChange: (draft: WorkflowRecipeDraftDto) => void;
}) {
  const addNode = () => {
    if (!defaultProfile) return;
    const nodeId = `node-${crypto.randomUUID()}`;
    const capabilities = defaultProfile.allowedCapabilities;
    const node: WorkflowAuthoringNodeDto = {
      nodeId,
      name: `Node ${draft.nodes.length + 1}`,
      positionX: draft.nodes.length * 240,
      positionY: 0,
      capabilityProfileId: defaultProfile.capabilityProfileId,
      nodeProfile: {
        contractVersion: 1,
        allowedCapabilities: capabilities,
        pinnedDefaults: defaultsWithinCapabilities(capabilities, runtimeLockedSelections),
      },
      initialPrompt: null,
      agentIdentityId: null,
    };
    onChange({
      ...draft,
      startingNodeId: draft.startingNodeId ?? nodeId,
      nodes: [...draft.nodes, node],
    });
    onSelect({ kind: 'node', id: nodeId });
  };
  const addConnection = () => {
    const first = draft.nodes[0];
    if (!first) return;
    const connectionId = `connection-${crypto.randomUUID()}`;
    const connection: WorkflowAuthoringConnectionDto = {
      connectionId,
      name: `Connection ${draft.connections.length + 1}`,
      sourceNodeId: first.nodeId,
      destinationNodeId: draft.nodes[1]?.nodeId ?? first.nodeId,
      trigger: { kind: 'invocation_completed' },
      promptInputs: [{ kind: 'invocation_output' }],
      promptText: '',
      target: {
        cardinality: 'first',
        ordering: 'newest',
        running: 'any',
        createdBy: null,
        missing: 'create',
      },
    };
    onChange({ ...draft, connections: [...draft.connections, connection] });
    onSelect({ kind: 'connection', id: connectionId });
  };
  return (
    <aside className="workflow-outline">
      <section>
        <header>
          <h2>Nodes</h2>
          <button type="button" disabled={!defaultProfile} onClick={addNode}>
            <Plus size={14} /> Add
          </button>
        </header>
        {!defaultProfile ? <p>Create a Capability Profile before adding nodes.</p> : null}
        {draft.nodes.map((node) => (
          <div className="workflow-outline__row" key={node.nodeId}>
            <button
              type="button"
              className={
                selection.kind === 'node' && selection.id === node.nodeId
                  ? 'is-selected'
                  : undefined
              }
              onClick={() => onSelect({ kind: 'node', id: node.nodeId })}
            >
              <strong>{node.name}</strong>
              <span>{node.nodeId}</span>
            </button>
            <label title="Starting node">
              <input
                type="radio"
                name="starting-node"
                checked={draft.startingNodeId === node.nodeId}
                onChange={() => onChange({ ...draft, startingNodeId: node.nodeId })}
              />
            </label>
            <button
              type="button"
              aria-label={`Remove ${node.name}`}
              onClick={() => {
                const nodes = draft.nodes.filter((candidate) => candidate.nodeId !== node.nodeId);
                onChange({
                  ...draft,
                  startingNodeId:
                    draft.startingNodeId === node.nodeId
                      ? (nodes[0]?.nodeId ?? null)
                      : draft.startingNodeId,
                  nodes,
                  connections: draft.connections.filter(
                    (connection) =>
                      connection.sourceNodeId !== node.nodeId &&
                      connection.destinationNodeId !== node.nodeId,
                  ),
                });
                onSelect({ kind: 'node', id: nodes[0]?.nodeId ?? null });
              }}
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
      </section>
      <section>
        <header>
          <h2>Connections</h2>
          <button type="button" disabled={draft.nodes.length === 0} onClick={addConnection}>
            <Plus size={14} /> Add
          </button>
        </header>
        {draft.connections.map((connection) => (
          <div className="workflow-outline__row" key={connection.connectionId}>
            <button
              type="button"
              className={
                selection.kind === 'connection' && selection.id === connection.connectionId
                  ? 'is-selected'
                  : undefined
              }
              onClick={() => onSelect({ kind: 'connection', id: connection.connectionId })}
            >
              <strong>{connection.name}</strong>
              <span>
                {connection.sourceNodeId} → {connection.destinationNodeId}
              </span>
            </button>
            <button
              type="button"
              aria-label={`Remove ${connection.name}`}
              onClick={() => {
                onChange({
                  ...draft,
                  connections: draft.connections.filter(
                    (candidate) => candidate.connectionId !== connection.connectionId,
                  ),
                });
                onSelect({ kind: 'node', id: draft.startingNodeId });
              }}
            >
              <Trash2 size={14} />
            </button>
          </div>
        ))}
      </section>
      <button
        type="button"
        className={
          selection.kind === 'run' ? 'workflow-outline__run is-selected' : 'workflow-outline__run'
        }
        onClick={() => onSelect({ kind: 'run', id: null })}
      >
        Compile and run
      </button>
    </aside>
  );
}
