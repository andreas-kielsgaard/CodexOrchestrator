import { ChevronDown, ChevronUp, Plus, Trash2 } from 'lucide-react';
import '../sessionEvents/sessionEvents.css';
import type {
  WorkflowAuthoringNodeDto,
  WorkflowConnectionPromptInputDto as Input,
  OtpOutputDto,
} from '../../application/workflowAuthoring';

const names: Record<Input['kind'], string> = {
  output_field: 'Output field',
  node_files: 'Files associated with node',
  file_content: 'File content',
};

export function WorkflowPromptInputsEditor({
  value,
  nodes,
  output,
  onChange,
}: {
  readonly value: readonly Input[];
  readonly nodes: readonly WorkflowAuthoringNodeDto[];
  readonly output?: OtpOutputDto;
  readonly onChange: (value: readonly Input[]) => void;
}) {
  const fields = Object.keys(output?.schema.properties ?? {});
  const kinds: Input['kind'][] = [
    ...(fields.length ? ['output_field' as const] : []),
    'node_files',
    'file_content',
  ];
  const make = (kind: Input['kind']): Input => {
    switch (kind) {
      case 'output_field':
        return { kind, field: fields[0] ?? '' };
      case 'node_files':
        return { kind, nodeId: nodes[0]?.nodeId ?? '', association: 'either' };
      case 'file_content':
        return { kind, path: '' };
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
            {input.kind === 'output_field' && (
              <label className="session-event-editor__field">
                Output field
                <select
                  value={input.field}
                  onChange={(event) => update(index, { ...input, field: event.target.value })}
                >
                  {!fields.includes(input.field) && (
                    <option value={input.field}>Select an available field</option>
                  )}
                  {fields.map((field) => (
                    <option key={field} value={field}>
                      {field}
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
            {input.kind === 'file_content' && (
              <label className="session-event-editor__field">
                File path
                <input
                  value={input.path}
                  onChange={(event) =>
                    update(index, {
                      ...input,
                      path: event.target.value,
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
