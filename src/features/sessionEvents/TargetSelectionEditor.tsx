import { emptyLogicalAddress, emptyReference } from './defaults';
import { LogicalAddressFields, ReferenceIdentityFields } from './ReferenceIdentityFields';
import type {
  MissingTargetPolicy,
  RunningFilter,
  SessionTarget,
  TargetCardinality,
  TargetOrdering,
  TargetSelection,
} from './types';
import './sessionEvents.css';

export interface TargetSelectionEditorProps {
  readonly value: TargetSelection;
  readonly disabled?: boolean;
  /** Higher-level editors may derive the logical target and expose only matching policy. */
  readonly hideTargetAddress?: boolean;
  readonly onChange: (value: TargetSelection) => void;
}

function targetForKind(kind: SessionTarget['kind']): SessionTarget {
  return kind === 'logical'
    ? { kind, address: emptyLogicalAddress() }
    : { kind, session: emptyReference('session') };
}

export function TargetSelectionEditor({
  value,
  disabled,
  hideTargetAddress = false,
  onChange,
}: TargetSelectionEditorProps) {
  const setTargetKind = (kind: SessionTarget['kind']) => {
    onChange({
      ...value,
      target: targetForKind(kind),
      missing: kind === 'exact' && value.missing === 'create' ? 'fail' : value.missing,
    });
  };

  return (
    <fieldset className="session-event-editor" disabled={disabled}>
      <legend>Target Session</legend>
      <div className="session-event-editor__grid">
        {!hideTargetAddress ? (
          <label className="session-event-editor__field">
            <span>Address type</span>
            <select
              value={value.target.kind}
              onChange={(event) => setTargetKind(event.target.value as SessionTarget['kind'])}
            >
              <option value="logical">Logical address</option>
              <option value="exact">Exact Session</option>
            </select>
          </label>
        ) : null}
        <label className="session-event-editor__field">
          <span>Match</span>
          <select
            value={value.cardinality}
            onChange={(event) =>
              onChange({ ...value, cardinality: event.target.value as TargetCardinality })
            }
          >
            <option value="first">First matching Session</option>
            <option value="all">All matching Sessions</option>
          </select>
        </label>
        <label className="session-event-editor__field">
          <span>Order matching Sessions by</span>
          <select
            value={value.ordering}
            onChange={(event) =>
              onChange({ ...value, ordering: event.target.value as TargetOrdering })
            }
          >
            <option value="newest">Newest created</option>
            <option value="last_addressed">Last addressed</option>
          </select>
        </label>
        <label className="session-event-editor__field">
          <span>Running state</span>
          <select
            value={value.running}
            onChange={(event) =>
              onChange({ ...value, running: event.target.value as RunningFilter })
            }
          >
            <option value="any">Any</option>
            <option value="running_only">Currently running</option>
            <option value="not_running">Not running</option>
          </select>
        </label>
      </div>

      {!hideTargetAddress && value.target.kind === 'logical' ? (
        <LogicalAddressFields
          legend="Logical Session address"
          value={value.target.address}
          onChange={(address) => onChange({ ...value, target: { kind: 'logical', address } })}
        />
      ) : !hideTargetAddress && value.target.kind === 'exact' ? (
        <ReferenceIdentityFields
          legend="Exact Session"
          value={value.target.session}
          onChange={(session) => onChange({ ...value, target: { kind: 'exact', session } })}
        />
      ) : null}

      <div className="session-event-editor__nested">
        <label className="session-event-editor__check">
          <input
            type="checkbox"
            checked={value.createdBy !== null}
            onChange={(event) =>
              onChange({
                ...value,
                createdBy: event.target.checked
                  ? { event: emptyReference('event_group'), session: null }
                  : null,
              })
            }
          />
          Limit to Sessions created by a recorded source
        </label>
        {value.createdBy && (
          <div className="session-event-editor__nested">
            <label className="session-event-editor__check">
              <input
                type="checkbox"
                checked={value.createdBy.event !== null}
                onChange={(event) =>
                  onChange({
                    ...value,
                    createdBy: {
                      ...value.createdBy!,
                      event: event.target.checked ? emptyReference('event_group') : null,
                    },
                  })
                }
              />
              Match creating event
            </label>
            {value.createdBy.event && (
              <ReferenceIdentityFields
                legend="Creating event"
                value={value.createdBy.event}
                onChange={(event) =>
                  onChange({ ...value, createdBy: { ...value.createdBy!, event } })
                }
              />
            )}
            <label className="session-event-editor__check">
              <input
                type="checkbox"
                checked={value.createdBy.session !== null}
                onChange={(event) =>
                  onChange({
                    ...value,
                    createdBy: {
                      ...value.createdBy!,
                      session: event.target.checked ? emptyReference('session') : null,
                    },
                  })
                }
              />
              Match creating Session
            </label>
            {value.createdBy.session && (
              <ReferenceIdentityFields
                legend="Creating Session"
                value={value.createdBy.session}
                onChange={(session) =>
                  onChange({ ...value, createdBy: { ...value.createdBy!, session } })
                }
              />
            )}
          </div>
        )}
      </div>

      <label className="session-event-editor__field">
        <span>If no Session matches</span>
        <select
          value={value.missing}
          onChange={(event) =>
            onChange({ ...value, missing: event.target.value as MissingTargetPolicy })
          }
        >
          <option value="create" disabled={value.target.kind !== 'logical'}>
            Create a Session
          </option>
          <option value="fail">Fail the event</option>
          <option value="noop">Do nothing</option>
        </select>
      </label>
      {value.target.kind !== 'logical' && (
        <p className="session-event-editor__hint">Creating a Session requires a logical address.</p>
      )}
    </fieldset>
  );
}
