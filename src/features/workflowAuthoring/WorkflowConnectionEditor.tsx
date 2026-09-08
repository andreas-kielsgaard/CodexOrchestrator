import type {
  SessionEventTriggerBindingDto,
  TargetSelectionDto,
} from '../../application/sessionEvents';
import type {
  WorkflowAuthoringConnectionDto,
  WorkflowAuthoringNodeDto,
  WorkflowTriggerCapabilityDto,
  WorkflowConnectionTriggerDto,
} from '../../application/workflowAuthoring';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { TargetSelectionEditor, TriggerBindingEditor } from '../sessionEvents';
import { WorkflowPromptInputsEditor } from './WorkflowPromptInputsEditor';

export function WorkflowConnectionEditor({
  connection,
  nodes,
  triggerCapabilities = [],
  onChange,
}: {
  readonly connection: WorkflowAuthoringConnectionDto;
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly triggerCapabilities?: readonly WorkflowTriggerCapabilityDto[];
  readonly onChange: (connection: WorkflowAuthoringConnectionDto) => void;
}) {
  const trigger = workflowTriggerToEventTrigger(connection.trigger);
  const target = workflowTargetToEventTarget(connection);
  const capability = triggerCapabilities.find(
    (item) =>
      connection.trigger.kind === 'mcp_call' &&
      connection.trigger.server.namespace === 'mcp' &&
      connection.trigger.server.kind === 'server' &&
      connection.trigger.tool.namespace === 'mcp' &&
      connection.trigger.tool.kind === 'tool' &&
      connection.trigger.server.id === item.server &&
      connection.trigger.tool.id === item.tool,
  );
  return (
    <div className="workflow-connection-editor">
      <header>
        <p>Workflow connection</p>
        <h1>{connection.name}</h1>
        <span>Compiles into a Session Event definition.</span>
      </header>
      <CollapsibleSection
        title="Connection"
        description="The visual edge determines the source and destination node."
        className="execution-configuration__section"
      >
        <div className="workflow-connection-editor__grid">
          <label>
            <span>Name</span>
            <input
              value={connection.name}
              onChange={(event) => onChange({ ...connection, name: event.currentTarget.value })}
            />
          </label>
          <label>
            <span>Source node</span>
            <select
              value={connection.sourceNodeId}
              onChange={(event) =>
                onChange({ ...connection, sourceNodeId: event.currentTarget.value })
              }
            >
              {nodes.map((node) => (
                <option key={node.nodeId} value={node.nodeId}>
                  {node.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Destination node</span>
            <select
              value={connection.destinationNodeId}
              onChange={(event) =>
                onChange({ ...connection, destinationNodeId: event.currentTarget.value })
              }
            >
              {nodes.map((node) => (
                <option key={node.nodeId} value={node.nodeId}>
                  {node.name}
                </option>
              ))}
            </select>
          </label>
        </div>
      </CollapsibleSection>
      <CollapsibleSection
        title="Trigger"
        description="The source node address is derived when the recipe compiles."
        className="execution-configuration__section"
      >
        <label>
          Workflow action
          <select
            value={capability?.id ?? ''}
            onChange={(event) => {
              const selected = triggerCapabilities.find((item) => item.id === event.target.value);
              if (selected)
                onChange({
                  ...connection,
                  trigger: {
                    kind: 'mcp_call',
                    server: { namespace: 'mcp', kind: 'server', id: selected.server },
                    tool: { namespace: 'mcp', kind: 'tool', id: selected.tool },
                  },
                });
              else onChange({ ...connection, trigger: { kind: 'invocation_completed' } });
            }}
          >
            <option value="">Other trigger</option>
            {triggerCapabilities.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
        </label>
        {!capability && (
          <TriggerBindingEditor
            value={trigger}
            hideInvocationSourceAddress
            allowedKinds={['invocation_completed', 'mcp_call', 'application_event']}
            onChange={(next) =>
              onChange({
                ...connection,
                trigger: eventTriggerToWorkflowTrigger(
                  next.kind === 'mcp_call' && !next.server.id && !next.tool.id
                    ? {
                        kind: 'mcp_call',
                        server: { namespace: 'mcp', kind: 'server', id: 'workflow_handoff' },
                        tool: { namespace: 'mcp', kind: 'tool', id: 'handoff_to_agent' },
                      }
                    : next,
                ),
              })
            }
          />
        )}
      </CollapsibleSection>
      <CollapsibleSection
        title="Prompt logic"
        description="Sources are materialized in order; fixed connection text is appended last."
        className="execution-configuration__section"
      >
        <WorkflowPromptInputsEditor
          value={connection.promptInputs}
          trigger={connection.trigger}
          nodes={nodes}
          capability={capability}
          onChange={(promptInputs) => onChange({ ...connection, promptInputs })}
        />
        <label className="workflow-connection-editor__prompt">
          <span>Fixed connection prompt</span>
          <textarea
            rows={5}
            value={connection.promptText}
            onChange={(event) => onChange({ ...connection, promptText: event.currentTarget.value })}
          />
        </label>
      </CollapsibleSection>
      <CollapsibleSection
        title="Session addressing"
        description={`The destination node “${nodes.find((node) => node.nodeId === connection.destinationNodeId)?.name ?? connection.destinationNodeId}” supplies the logical address.`}
        className="execution-configuration__section"
      >
        <TargetSelectionEditor
          value={target}
          hideTargetAddress
          onChange={(next) =>
            onChange({
              ...connection,
              target: {
                cardinality: next.cardinality,
                ordering: next.ordering,
                running: next.running,
                createdBy: next.createdBy,
                missing: next.missing,
              },
            })
          }
        />
      </CollapsibleSection>
    </div>
  );
}

function workflowTriggerToEventTrigger(
  trigger: WorkflowConnectionTriggerDto,
): SessionEventTriggerBindingDto {
  return trigger.kind === 'invocation_completed' ? { ...trigger, sourceAddress: null } : trigger;
}

function eventTriggerToWorkflowTrigger(
  trigger: SessionEventTriggerBindingDto,
): WorkflowConnectionTriggerDto {
  if (trigger.kind === 'user_request') return { kind: 'invocation_completed' };
  if (trigger.kind === 'invocation_completed') return { kind: trigger.kind };
  return trigger;
}

function workflowTargetToEventTarget(
  connection: WorkflowAuthoringConnectionDto,
): TargetSelectionDto {
  return {
    target: {
      kind: 'logical',
      address: {
        scope: {
          namespace: 'orchestrator.workflow_instances',
          kind: 'workflow_instance',
          id: 'derived-at-compile',
        },
        subject: {
          namespace: 'orchestrator.workflow_nodes',
          kind: 'workflow_node',
          id: connection.destinationNodeId,
        },
      },
    },
    ...connection.target,
  };
}
