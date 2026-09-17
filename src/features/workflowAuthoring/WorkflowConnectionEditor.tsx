import type {
  WorkflowAuthoringConnectionDto,
  WorkflowAuthoringNodeDto,
  OtpPackageDto,
} from '../../application/workflowAuthoring';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { WorkflowPromptInputsEditor } from './WorkflowPromptInputsEditor';
import { WorkflowTriggerPicker } from './WorkflowTriggerPicker';
import { WorkflowDestinationActionPicker } from './WorkflowDestinationActionPicker';
import { offeredActions, offeredOutputs, capabilityKey, outputKey } from './otpPresentation';

export function WorkflowConnectionEditor({
  connection,
  nodes,
  packages,
  onChange,
}: {
  readonly connection: WorkflowAuthoringConnectionDto;
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly packages: readonly OtpPackageDto[];
  readonly onChange: (connection: WorkflowAuthoringConnectionDto) => void;
}) {
  const outputs = offeredOutputs(packages);
  const actions = offeredActions(packages);
  const selectedOutput = outputs.find(
    (item) => outputKey(item.ref) === outputKey(connection.trigger),
  );
  const selectedAction = actions.find(
    (item) => capabilityKey(item.ref) === capabilityKey(connection.action),
  );
  return (
    <div className="workflow-connection-editor">
      <header>
        <p>Workflow connection</p>
        <h1>{connection.name}</h1>
      </header>
      <CollapsibleSection
        title="Connection"
        description="Choose the source and output node."
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
            <span>Output node</span>
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
        description="An output from the selected source node starts this connection."
        className="execution-configuration__section"
      >
        <WorkflowTriggerPicker connection={connection} packages={packages} onChange={onChange} />
        {selectedOutput && (
          <p>
            Offered fields:{' '}
            {Object.entries(selectedOutput.output.schema.properties ?? {})
              .map(([key, schema]) => key + ' (' + schema.type + ')')
              .join(', ')}
          </p>
        )}
      </CollapsibleSection>
      {selectedAction?.tool.entrypoint.kind === 'action' &&
        selectedAction.tool.entrypoint.usesPrompt !== false && (
          <CollapsibleSection
            title="Prompt logic"
            description="Include these inputs in order, followed by the fixed prompt."
            className="execution-configuration__section"
          >
            <WorkflowPromptInputsEditor
              value={connection.promptInputs}
              nodes={nodes}
              output={selectedOutput?.output}
              onChange={(promptInputs) => onChange({ ...connection, promptInputs })}
            />
            <label className="workflow-connection-editor__prompt">
              <span>Fixed connection prompt</span>
              <textarea
                rows={5}
                value={connection.promptText}
                onChange={(event) =>
                  onChange({ ...connection, promptText: event.currentTarget.value })
                }
              />
            </label>
          </CollapsibleSection>
        )}
      <CollapsibleSection
        title="Destination action"
        description="Choose what to do with Sessions of the output node."
        className="execution-configuration__section"
      >
        <WorkflowDestinationActionPicker
          action={connection.action}
          configuration={connection.configuration}
          packages={packages}
          onChange={(action, configuration) => onChange({ ...connection, action, configuration })}
        />
      </CollapsibleSection>
    </div>
  );
}
