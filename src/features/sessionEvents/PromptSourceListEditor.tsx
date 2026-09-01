import { ChevronDown, ChevronUp, Plus, Trash2 } from 'lucide-react';
import { emptyReference } from './defaults';
import { ReferenceIdentityFields } from './ReferenceIdentityFields';
import type { PromptSourceDefinition } from './types';
import './sessionEvents.css';

export interface PromptSourceListEditorProps {
  readonly value: readonly PromptSourceDefinition[];
  readonly label?: string;
  readonly description?: string;
  readonly disabled?: boolean;
  readonly onChange: (value: PromptSourceDefinition[]) => void;
}

type PromptSourceKind = PromptSourceDefinition['kind'];

function promptSourceForKind(kind: PromptSourceKind): PromptSourceDefinition {
  switch (kind) {
    case 'literal':
      return { kind, text: '' };
    case 'user_request_text':
    case 'invocation_output':
      return { kind };
    case 'mcp_argument':
      return { kind, name: '' };
    case 'application_event_field':
      return { kind, field: '' };
    case 'referenced_content':
      return { kind, reference: emptyReference('content') };
  }
}

function promptSourceName(kind: PromptSourceKind): string {
  switch (kind) {
    case 'literal':
      return 'Fixed text';
    case 'user_request_text':
      return 'User request text';
    case 'invocation_output':
      return 'Invocation output';
    case 'mcp_argument':
      return 'MCP argument';
    case 'application_event_field':
      return 'Application event field';
    case 'referenced_content':
      return 'Referenced content';
  }
}

export function PromptSourceListEditor({
  value,
  label = 'Prompt sources',
  description,
  disabled,
  onChange,
}: PromptSourceListEditorProps) {
  const updateAt = (index: number, source: PromptSourceDefinition) => {
    onChange(value.map((current, position) => (position === index ? source : current)));
  };
  const move = (index: number, offset: -1 | 1) => {
    const reordered = [...value];
    const [source] = reordered.splice(index, 1);
    reordered.splice(index + offset, 0, source);
    onChange(reordered);
  };

  return (
    <fieldset className="session-event-editor" disabled={disabled}>
      <legend>{label}</legend>
      {description && <p className="session-event-editor__description">{description}</p>}
      {value.length === 0 && (
        <p className="session-event-editor__empty">No prompt sources configured.</p>
      )}
      <ol className="prompt-source-list">
        {value.map((source, index) => (
          <li className="prompt-source-list__item" key={index}>
            <div className="prompt-source-list__header">
              <strong>
                {index + 1}. {promptSourceName(source.kind)}
              </strong>
              <span className="prompt-source-list__actions">
                <button
                  type="button"
                  aria-label={`Move prompt source ${index + 1} up`}
                  disabled={disabled || index === 0}
                  onClick={() => move(index, -1)}
                >
                  <ChevronUp size={15} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  aria-label={`Move prompt source ${index + 1} down`}
                  disabled={disabled || index === value.length - 1}
                  onClick={() => move(index, 1)}
                >
                  <ChevronDown size={15} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  aria-label={`Remove prompt source ${index + 1}`}
                  disabled={disabled}
                  onClick={() => onChange(value.filter((_, position) => position !== index))}
                >
                  <Trash2 size={15} aria-hidden="true" />
                </button>
              </span>
            </div>
            <label className="session-event-editor__field">
              <span>Source type</span>
              <select
                value={source.kind}
                onChange={(event) =>
                  updateAt(index, promptSourceForKind(event.target.value as PromptSourceKind))
                }
              >
                <option value="literal">Fixed text</option>
                <option value="user_request_text">User request text</option>
                <option value="invocation_output">Invocation output</option>
                <option value="mcp_argument">MCP argument</option>
                <option value="application_event_field">Application event field</option>
                <option value="referenced_content">Referenced content</option>
              </select>
            </label>
            {source.kind === 'literal' && (
              <label className="session-event-editor__field">
                <span>Text</span>
                <textarea
                  rows={3}
                  value={source.text}
                  onChange={(event) => updateAt(index, { ...source, text: event.target.value })}
                />
              </label>
            )}
            {source.kind === 'mcp_argument' && (
              <label className="session-event-editor__field">
                <span>Argument name</span>
                <input
                  value={source.name}
                  onChange={(event) => updateAt(index, { ...source, name: event.target.value })}
                />
              </label>
            )}
            {source.kind === 'application_event_field' && (
              <label className="session-event-editor__field">
                <span>Field name</span>
                <input
                  value={source.field}
                  onChange={(event) => updateAt(index, { ...source, field: event.target.value })}
                />
              </label>
            )}
            {source.kind === 'referenced_content' && (
              <ReferenceIdentityFields
                legend="Content reference"
                value={source.reference}
                onChange={(reference) => updateAt(index, { ...source, reference })}
              />
            )}
          </li>
        ))}
      </ol>
      <button
        className="session-event-editor__add"
        type="button"
        disabled={disabled}
        onClick={() => onChange([...value, promptSourceForKind('literal')])}
      >
        <Plus size={15} aria-hidden="true" />
        Add prompt source
      </button>
    </fieldset>
  );
}
