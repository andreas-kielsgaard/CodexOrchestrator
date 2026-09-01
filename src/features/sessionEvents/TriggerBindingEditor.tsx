import { emptyLogicalAddress, emptyReference } from './defaults';
import { LogicalAddressFields, ReferenceIdentityFields } from './ReferenceIdentityFields';
import type { SessionEventTriggerBinding } from './types';
import './sessionEvents.css';

export interface TriggerBindingEditorProps {
  readonly value: SessionEventTriggerBinding;
  readonly disabled?: boolean;
  readonly onChange: (value: SessionEventTriggerBinding) => void;
}

type TriggerKind = SessionEventTriggerBinding['kind'];

function triggerForKind(kind: TriggerKind): SessionEventTriggerBinding {
  switch (kind) {
    case 'user_request':
      return { kind };
    case 'invocation_completed':
      return { kind, sourceAddress: null };
    case 'mcp_call':
      return {
        kind,
        server: emptyReference('mcp_server'),
        tool: emptyReference('mcp_tool'),
      };
    case 'application_event':
      return { kind, eventKind: emptyReference('event_kind') };
    case 'event_group_completed':
      return { kind, sourceDefinition: emptyReference('event_definition') };
  }
}

export function TriggerBindingEditor({ value, disabled, onChange }: TriggerBindingEditorProps) {
  return (
    <fieldset className="session-event-editor" disabled={disabled}>
      <legend>Trigger</legend>
      <label className="session-event-editor__field">
        <span>Trigger type</span>
        <select
          value={value.kind}
          onChange={(event) => onChange(triggerForKind(event.target.value as TriggerKind))}
        >
          <option value="user_request">User request</option>
          <option value="invocation_completed">Invocation completed</option>
          <option value="mcp_call">MCP call</option>
          <option value="application_event">Application event</option>
          <option value="event_group_completed">Event group completed</option>
        </select>
      </label>

      {value.kind === 'invocation_completed' && (
        <div className="session-event-editor__nested">
          <label className="session-event-editor__check">
            <input
              type="checkbox"
              checked={value.sourceAddress !== null}
              onChange={(event) =>
                onChange({
                  ...value,
                  sourceAddress: event.target.checked ? emptyLogicalAddress() : null,
                })
              }
            />
            Limit to a source Session address
          </label>
          {value.sourceAddress && (
            <LogicalAddressFields
              legend="Source Session address"
              value={value.sourceAddress}
              onChange={(sourceAddress) => onChange({ ...value, sourceAddress })}
            />
          )}
        </div>
      )}

      {value.kind === 'mcp_call' && (
        <div className="session-event-editor__pair">
          <ReferenceIdentityFields
            legend="MCP server"
            value={value.server}
            onChange={(server) => onChange({ ...value, server })}
          />
          <ReferenceIdentityFields
            legend="MCP tool"
            value={value.tool}
            onChange={(tool) => onChange({ ...value, tool })}
          />
        </div>
      )}

      {value.kind === 'application_event' && (
        <ReferenceIdentityFields
          legend="Application event kind"
          value={value.eventKind}
          onChange={(eventKind) => onChange({ ...value, eventKind })}
        />
      )}

      {value.kind === 'event_group_completed' && (
        <ReferenceIdentityFields
          legend="Source event definition"
          value={value.sourceDefinition}
          onChange={(sourceDefinition) => onChange({ ...value, sourceDefinition })}
        />
      )}
    </fieldset>
  );
}
