import { ChevronDown, ChevronUp, Plus, Trash2 } from 'lucide-react';
import '../sessionEvents/sessionEvents.css';
import type {
  WorkflowAuthoringNodeDto,
  WorkflowConnectionPromptInputDto as Input,
  WorkflowConnectionTriggerDto,
  WorkflowTriggerCapabilityDto,
} from '../../application/workflowAuthoring';

const names: Record<Input['kind'], string> = {
  trigger_field: 'Trigger field',
  node_files: 'Files associated with node',
  invocation_output: 'Invocation output',
  mcp_argument: 'MCP argument',
  application_event_field: 'Application event field',
  referenced_content: 'File content',
};

export function WorkflowPromptInputsEditor({
  value,
  trigger,
  nodes,
  capability,
  onChange,
}: {
  readonly value: readonly Input[];
  readonly trigger: WorkflowConnectionTriggerDto;
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly capability?: WorkflowTriggerCapabilityDto;
  readonly onChange: (value: readonly Input[]) => void;
}) {
  const kinds: Input['kind'][] = [
    ...(capability ? ['trigger_field' as const] : []),
    ...(trigger.kind === 'invocation_completed' ? ['invocation_output' as const] : []),
    ...(trigger.kind === 'mcp_call' && !capability ? ['mcp_argument' as const] : []),
    ...(trigger.kind === 'application_event' ? ['application_event_field' as const] : []),
    'node_files',
    'referenced_content',
  ];
  const make = (kind: Input['kind']): Input => {
    switch (kind) {
      case 'trigger_field':
        return { kind, field: capability?.fields[0]?.name ?? '' };
      case 'node_files':
        return { kind, nodeId: nodes[0]?.nodeId ?? '', association: 'either' };
      case 'mcp_argument':
        return { kind, name: '' };
      case 'application_event_field':
        return { kind, field: '' };
      case 'referenced_content':
        return { kind, reference: { namespace: 'file', kind: 'path', id: '' } };
      case 'invocation_output':
        return { kind };
    }
  };
  const update = (index: number, input: Input) =>
    onChange(value.map((entry, i) => (i === index ? input : entry)));
  const move = (index: number, offset: number) => {
    const next = [...value];
    const [entry] = next.splice(index, 1);
    next.splice(index + offset, 0, entry);
    onChange(next);
  };
  return (
    <fieldset className="session-event-editor">
      <legend>Prompt sources</legend>
      {value.length === 0 && <p>No prompt sources configured.</p>}
      <ol className="prompt-source-list">
        {value.map((input, index) => (
          <li key={index} className="prompt-source-list__item">
            <div className="prompt-source-list__header">
              <strong>
                {index + 1}. {names[input.kind]}
              </strong>
              <span className="prompt-source-list__actions">
                <button
                  type="button"
                  aria-label={`Move prompt source ${index + 1} up`}
                  disabled={index === 0}
                  onClick={() => move(index, -1)}
                >
                  <ChevronUp size={15} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  aria-label={`Move prompt source ${index + 1} down`}
                  disabled={index === value.length - 1}
                  onClick={() => move(index, 1)}
                >
                  <ChevronDown size={15} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  aria-label={`Remove prompt source ${index + 1}`}
                  onClick={() => onChange(value.filter((_, i) => i !== index))}
                >
                  <Trash2 size={14} aria-hidden="true" />
                </button>
              </span>
            </div>
            <label className="session-event-editor__field">
              Source type
              <select
                value={input.kind}
                onChange={(event) => update(index, make(event.target.value as Input['kind']))}
              >
                {!kinds.includes(input.kind) && (
                  <option value={input.kind}>
                    {names[input.kind]} (not available for this trigger)
                  </option>
                )}
                {kinds.map((kind) => (
                  <option key={kind} value={kind}>
                    {names[kind]}
                  </option>
                ))}
              </select>
            </label>
            {input.kind === 'trigger_field' && (
              <label className="session-event-editor__field">
                Trigger field
                <select
                  value={input.field}
                  onChange={(event) => update(index, { ...input, field: event.target.value })}
                >
                  {!capability?.fields.some((field) => field.name === input.field) && (
                    <option value={input.field}>Select an available field</option>
                  )}
                  {capability?.fields.map((field) => (
                    <option key={field.name} value={field.name}>
                      {field.label}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {input.kind === 'node_files' && (
              <>
                <label className="session-event-editor__field">
                  Include files from node
                  <select
                    value={input.nodeId}
                    onChange={(event) => update(index, { ...input, nodeId: event.target.value })}
                  >
                    {!nodes.some((node) => node.nodeId === input.nodeId) && (
                      <option value={input.nodeId}>Select a node</option>
                    )}
                    {nodes.map((node) => (
                      <option key={node.nodeId} value={node.nodeId}>
                        {node.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="session-event-editor__field">
                  File association
                  <select
                    value={input.association}
                    onChange={(event) =>
                      update(index, {
                        ...input,
                        association: event.target.value as 'created' | 'edited' | 'either',
                      })
                    }
                  >
                    <option value="created">Created</option>
                    <option value="edited">Edited</option>
                    <option value="either">Created or edited</option>
                  </select>
                </label>
              </>
            )}
            {input.kind === 'mcp_argument' && (
              <label className="session-event-editor__field">
                Argument name
                <input
                  value={input.name}
                  onChange={(event) => update(index, { ...input, name: event.target.value })}
                />
              </label>
            )}
            {input.kind === 'application_event_field' && (
              <label className="session-event-editor__field">
                Event field
                <input
                  value={input.field}
                  onChange={(event) => update(index, { ...input, field: event.target.value })}
                />
              </label>
            )}
            {input.kind === 'referenced_content' && (
              <label className="session-event-editor__field">
                File path
                <input
                  value={input.reference.id}
                  onChange={(event) =>
                    update(index, {
                      ...input,
                      reference: { namespace: 'file', kind: 'path', id: event.target.value },
                    })
                  }
                />
              </label>
            )}
          </li>
        ))}
      </ol>
      <button
        className="session-event-editor__add"
        type="button"
        onClick={() => onChange([...value, make(kinds[0])])}
      >
        <Plus size={15} aria-hidden="true" />
        Add prompt source
      </button>
    </fieldset>
  );
}
