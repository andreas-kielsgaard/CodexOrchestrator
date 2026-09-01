import type {
  SessionEventTriggerBindingDto,
  TargetSelectionDto,
} from '../../application/sessionEvents';
import type {
  WorkflowAuthoringConnectionDto,
  WorkflowAuthoringNodeDto,
  WorkflowConnectionPromptInputDto,
  WorkflowConnectionTriggerDto,
} from '../../application/workflowAuthoring';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import {
  PromptSourceListEditor,
  TargetSelectionEditor,
  TriggerBindingEditor,
} from '../sessionEvents';

export function WorkflowConnectionEditor({
  connection,
  nodes,
  onChange,
}: {
  readonly connection: WorkflowAuthoringConnectionDto;
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly onChange: (connection: WorkflowAuthoringConnectionDto) => void;
}) {
  const trigger = workflowTriggerToEventTrigger(connection.trigger);
  const target = workflowTargetToEventTarget(connection);
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
        <TriggerBindingEditor
          value={trigger}
          hideInvocationSourceAddress
          allowedKinds={[
            'invocation_completed',
            'mcp_call',
            'application_event',
            'event_group_completed',
          ]}
          onChange={(next) =>
            onChange({ ...connection, trigger: eventTriggerToWorkflowTrigger(next) })
          }
        />
      </CollapsibleSection>
      <CollapsibleSection
        title="Prompt logic"
        description="Sources are materialized in order; fixed connection text is appended last."
        className="execution-configuration__section"
      >
        <PromptSourceListEditor
          value={connection.promptInputs}
          allowedKinds={[
            'invocation_output',
            'mcp_argument',
            'application_event_field',
            'referenced_content',
          ]}
          onChange={(value) =>
            onChange({
              ...connection,
              promptInputs: value as readonly WorkflowConnectionPromptInputDto[],
            })
          }
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
