import { Plus, Trash2 } from 'lucide-react';

type FilterOperation = 'include' | 'exclude';
type SessionFilter =
  | { readonly kind: 'idle_sessions'; readonly operation: FilterOperation }
  | {
      readonly kind: 'created_by_event';
      readonly operation: FilterOperation;
      readonly eventId: string;
    }
  | {
      readonly kind: 'created_by_session';
      readonly operation: FilterOperation;
      readonly sessionId: string;
    };
type SessionFilterKind = SessionFilter['kind'];

const filterLabels: Record<SessionFilterKind, string> = {
  idle_sessions: 'Idle sessions',
  created_by_event: 'Created by event',
  created_by_session: 'Created by session',
};

function readFilters(value: unknown): readonly SessionFilter[] {
  if (!Array.isArray(value)) return [];
  return value.filter((candidate): candidate is SessionFilter => {
    if (!candidate || typeof candidate !== 'object') return false;
    const record = candidate as Record<string, unknown>;
    return (
      ['idle_sessions', 'created_by_event', 'created_by_session'].includes(String(record.kind)) &&
      ['include', 'exclude'].includes(String(record.operation))
    );
  });
}

function createFilter(kind: SessionFilterKind): SessionFilter {
  if (kind === 'created_by_event') return { kind, operation: 'include', eventId: '' };
  if (kind === 'created_by_session') return { kind, operation: 'include', sessionId: '' };
  return { kind, operation: 'include' };
}

export function WorkflowPromptAgentConfigurationEditor({
  value,
  onChange,
}: {
  readonly value: Readonly<Record<string, unknown>>;
  onChange(value: Readonly<Record<string, unknown>>): void;
}) {
  const current = (key: string, fallback: string) => String(value[key] ?? fallback);
  const mode = current('mode', 'select');
  const filters = readFilters(value.filters);
  const commit = (patch: Readonly<Record<string, unknown>>) => {
    const rest: Record<string, unknown> = { ...value };
    delete rest.running;
    delete rest.createdByEvent;
    delete rest.createdBySession;
    onChange({ ...rest, ...patch });
  };
  const updateFilter = (index: number, filter: SessionFilter) =>
    commit({ filters: filters.map((candidate, i) => (i === index ? filter : candidate)) });
  const availableKinds = (currentKind?: SessionFilterKind) =>
    (Object.keys(filterLabels) as SessionFilterKind[]).filter(
      (kind) => kind === currentKind || !filters.some((filter) => filter.kind === kind),
    );

  return (
    <div className="workflow-prompt-agent-configuration">
      <label className="session-event-editor__field">
        Session mode
        <select value={mode} onChange={(event) => commit({ mode: event.currentTarget.value })}>
          <option value="select">Continue a session</option>
          <option value="new">Start a new session</option>
        </select>
      </label>
      {mode === 'select' ? (
        <>
          <div className="session-event-editor__field">
            <label>
              Sessions to prompt
              <select
                value={current('cardinality', 'first')}
                onChange={(event) => commit({ cardinality: event.currentTarget.value })}
              >
                <option value="first">One session</option>
                <option value="all">All sessions</option>
              </select>
            </label>
            <small>
              {current('cardinality', 'first') === 'all'
                ? 'Prompt every session that matches the filters.'
                : 'Prompt the first session after filtering and selection.'}
            </small>
          </div>
          <fieldset className="session-event-editor workflow-session-filters">
            <legend>Session filters</legend>
            {!filters.length ? <p>No session filters configured.</p> : null}
            <ol className="prompt-source-list">
              {filters.map((filter, index) => (
                <li className="prompt-source-list__item" key={filter.kind}>
                  <div className="workflow-session-filter__header">
                    <label className="session-event-editor__field">
                      Filter
                      <select
                        value={filter.kind}
                        onChange={(event) =>
                          updateFilter(
                            index,
                            createFilter(event.currentTarget.value as SessionFilterKind),
                          )
                        }
                      >
                        {availableKinds(filter.kind).map((kind) => (
                          <option value={kind} key={kind}>
                            {filterLabels[kind]}
                          </option>
                        ))}
                      </select>
                    </label>
                    <button
                      type="button"
                      aria-label={`Remove ${filterLabels[filter.kind]} filter`}
                      onClick={() =>
                        commit({ filters: filters.filter((_, candidate) => candidate !== index) })
                      }
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </div>
                  <div className="workflow-session-filter__operation" aria-label="Filter operation">
                    {(['include', 'exclude'] as const).map((operation) => (
                      <button
                        type="button"
                        key={operation}
                        aria-pressed={filter.operation === operation}
                        onClick={() => updateFilter(index, { ...filter, operation })}
                      >
                        {operation === 'include' ? 'Include' : 'Exclude'}
                      </button>
                    ))}
                  </div>
                  {filter.kind === 'created_by_event' ? (
                    <label className="session-event-editor__field">
                      Event ID
                      <input
                        value={filter.eventId}
                        onChange={(event) =>
                          updateFilter(index, { ...filter, eventId: event.currentTarget.value })
                        }
                      />
                    </label>
                  ) : null}
                  {filter.kind === 'created_by_session' ? (
                    <label className="session-event-editor__field">
                      Session ID
                      <input
                        value={filter.sessionId}
                        onChange={(event) =>
                          updateFilter(index, { ...filter, sessionId: event.currentTarget.value })
                        }
                      />
                    </label>
                  ) : null}
                </li>
              ))}
            </ol>
            {availableKinds().length ? (
              <label className="workflow-session-filter__add">
                <Plus size={15} aria-hidden="true" />
                <select
                  aria-label="Add session filter"
                  value=""
                  onChange={(event) => {
                    if (!event.currentTarget.value) return;
                    commit({
                      filters: [
                        ...filters,
                        createFilter(event.currentTarget.value as SessionFilterKind),
                      ],
                    });
                  }}
                >
                  <option value="">Add filter</option>
                  {availableKinds().map((kind) => (
                    <option value={kind} key={kind}>
                      {filterLabels[kind]}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </fieldset>
          <label className="session-event-editor__field">
            Session selection logic
            <select
              value={current('ordering', 'newest')}
              onChange={(event) => commit({ ordering: event.currentTarget.value })}
            >
              <option value="newest">Newest session</option>
              <option value="last_addressed">Most recently addressed session</option>
            </select>
          </label>
          <label className="session-event-editor__field">
            If no session matches
            <select
              value={current('missing', 'create')}
              onChange={(event) => commit({ missing: event.currentTarget.value })}
            >
              <option value="create">Create a new session</option>
              <option value="fail">Send failure event</option>
              <option value="noop">Do nothing</option>
            </select>
          </label>
        </>
      ) : null}
    </div>
  );
}
