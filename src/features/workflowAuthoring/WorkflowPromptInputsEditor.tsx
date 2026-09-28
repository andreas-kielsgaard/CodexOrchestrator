import { ChevronDown, ChevronUp, Plus, Trash2 } from 'lucide-react';
import '../sessionEvents/sessionEvents.css';
import type {
  WorkflowAuthoringNodeDto,
  WorkflowConnectionPromptInputDto as Input,
  OtpOutputDto,
} from '../../application/workflowAuthoring';

type PromptSourceChoice =
  | 'output_field'
  | 'node_files_edited'
  | 'node_files_created'
  | 'node_files_either'
  | 'file_content';

const names: Record<PromptSourceChoice, string> = {
  output_field: 'Source Node Output',
  node_files_edited: 'List of files edited by node',
  node_files_created: 'List of files created by node',
  node_files_either: 'List of files created or edited by node',
  file_content: 'File content',
};

export function promptSourceChoice(input: Input): PromptSourceChoice {
  if (input.kind !== 'node_files') return input.kind;
  return `node_files_${input.association}`;
}

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
  const choices: PromptSourceChoice[] = [
    ...(fields.length ? ['output_field' as const] : []),
    'node_files_edited',
    'node_files_created',
    'node_files_either',
    'file_content',
  ];
  const make = (choice: PromptSourceChoice): Input => {
    switch (choice) {
      case 'output_field':
        return { kind: 'output_field', field: fields[0] ?? '' };
      case 'node_files_edited':
        return { kind: 'node_files', nodeId: nodes[0]?.nodeId ?? '', association: 'edited' };
      case 'node_files_created':
        return { kind: 'node_files', nodeId: nodes[0]?.nodeId ?? '', association: 'created' };
      case 'node_files_either':
        return { kind: 'node_files', nodeId: nodes[0]?.nodeId ?? '', association: 'either' };
      case 'file_content':
        return { kind: 'file_content', path: '' };
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
                value={promptSourceChoice(input)}
                onChange={(event) => update(index, make(event.target.value as PromptSourceChoice))}
              >
                {!choices.includes(promptSourceChoice(input)) && (
                  <option value={promptSourceChoice(input)}>
                    {names[promptSourceChoice(input)]} (not available for this trigger)
                  </option>
                )}
                {choices.map((choice) => (
                  <option key={choice} value={choice}>
                    {names[choice]}
                  </option>
                ))}
              </select>
            </label>
            {input.kind === 'output_field' && (
              <label className="session-event-editor__field">
                Field
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
        disabled={!choices.length}
        onClick={() => onChange([...value, make(choices[0])])}
      >
        <Plus size={15} aria-hidden="true" />
        Add prompt source
      </button>
    </fieldset>
  );
}
