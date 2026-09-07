import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  WorkflowInstanceClient,
  WorkflowInstanceDetails,
} from '../../application/workflowInstances';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessionProfiles';
import type {
  SessionEventQueryClient,
  SessionEventResultDto,
} from '../../application/sessionEvents';
import { ProfiledSessionPane } from '../agentSessions/ProfiledSessionPane';
import { EventDeliveryList } from '../sessionEvents';
import { CollapsibleSection } from '../../components/CollapsibleSection';

export function WorkflowInstancePanel({
  instanceId,
  client,
  sessionClient,
  profileClient,
  queryClient,
}: {
  readonly instanceId: string;
  readonly client: WorkflowInstanceClient;
  readonly sessionClient?: AgentSessionClient;
  readonly profileClient?: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
}) {
  const [details, setDetails] = useState<WorkflowInstanceDetails | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [nodeId, setNodeId] = useState('');
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<SessionEventResultDto | null>(null);
  const mounted = useRef(true);
  const request = useRef(0);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const load = useCallback(async () => {
    const ticket = ++request.current;
    try {
      const value = await client.load(instanceId);
      if (!mounted.current || ticket !== request.current) return;
      setDetails(value);
      setNodeId((current) => current || value.instance.recipe.startingNodeId || '');
    } catch (cause) {
      if (mounted.current && ticket === request.current) setError(String(cause));
    }
  }, [client, instanceId]);
  useEffect(() => {
    void load();
    let active = true;
    let stop: (() => void) | undefined;
    void client
      .subscribeChanged?.((changedId) => {
        if (changedId === instanceId) void load();
      })
      .then((unsubscribe) => {
        if (active) stop = unsubscribe;
        else unsubscribe();
      })
      .catch((cause) => active && setError(String(cause)));
    return () => {
      active = false;
      stop?.();
    };
  }, [load, client, instanceId]);
  if (!details)
    return (
      <section className="recipe-instance-panel">
        <p>{error ?? 'Loading instance…'}</p>
        <button onClick={() => void load()}>Refresh</button>
      </section>
    );
  const { instance } = details;
  return (
    <section className="recipe-instance-panel">
      <header>
        <div>
          <h1>{instance.name}</h1>
          <p>
            {instance.recipe.name} · pinned v{instance.recipe.revision}
          </p>
          <p>{instance.target.worktree.path}</p>
        </div>
        <button
          type="button"
          onClick={() => {
            setError(null);
            void load();
          }}
        >
          Refresh instance
        </button>
      </header>
      {error ? <p role="alert">{error}</p> : null}
      <div className="recipe-instance-panel__body">
        <aside>
          <h2>Sessions</h2>
          {details.sessions.length === 0 ? (
            <p>No Sessions yet. Send a request to a node to start.</p>
          ) : null}
          {details.sessions.map((entry) => (
            <button
              type="button"
              key={entry.session.id}
              aria-pressed={selected === entry.session.id}
              onClick={() => setSelected(entry.session.id)}
            >
              {instance.recipe.nodes.find(
                (node) => node.nodeId === entry.logicalAddress?.subject.id,
              )?.name ?? entry.session.id}{' '}
              · {entry.running ? 'Running' : 'Idle'}
              <small>{entry.session.id}</small>
            </button>
          ))}
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (busy || !text.trim()) return;
              setBusy(true);
              setError(null);
              void client
                .messageNode({
                  instanceId,
                  recipeId: instance.recipe.recipeId,
                  nodeId: nodeId || null,
                  text,
                })
                .then(async (value) => {
                  if (!mounted.current) return;
                  setResult(value);
                  setText('');
                  setSelected(value.deliveries[0]?.targetSession.id ?? null);
                  await load();
                })
                .catch((cause) => mounted.current && setError(String(cause)))
                .finally(() => mounted.current && setBusy(false));
            }}
          >
            <h2>Send to node</h2>
            <label>
              Node
              <select value={nodeId} onChange={(event) => setNodeId(event.currentTarget.value)}>
                {instance.recipe.nodes.map((node) => (
                  <option key={node.nodeId} value={node.nodeId}>
                    {node.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Request
              <textarea
                value={text}
                rows={4}
                onChange={(event) => setText(event.currentTarget.value)}
              />
            </label>
            <button disabled={busy || !text.trim()}>{busy ? 'Sending…' : 'Send request'}</button>
            <small>Uses this instance's Workflow. Creates a Session if needed.</small>
          </form>
          <CollapsibleSection title="Handoffs" defaultExpanded={false}>
            {details.attempts.map((attempt) => (
              <div key={attempt.id}>
                <strong>{attempt.definitionRef.id}</strong>
                <p>
                  {attempt.error ??
                    (attempt.eventGroup ? 'Delivery recorded' : 'Preparing delivery')}
                </p>
                {attempt.eventGroup && queryClient ? (
                  <button
                    type="button"
                    onClick={() => {
                      void queryClient.loadRecordedEvent(attempt.eventGroup!).then(
                        (value) => mounted.current && setResult(value),
                        (cause) => mounted.current && setError(String(cause)),
                      );
                    }}
                  >
                    Show delivery
                  </button>
                ) : null}
              </div>
            ))}
          </CollapsibleSection>
          {result ? <EventDeliveryList deliveries={result.deliveries} /> : null}
        </aside>
        <div className="recipe-instance-panel__conversation">
          {selected && sessionClient && profileClient ? (
            <ProfiledSessionPane
              key={selected}
              sessionId={selected}
              client={sessionClient}
              profileClient={profileClient}
              queryClient={queryClient}
            />
          ) : (
            <p>Select a Session to open its conversation.</p>
          )}
        </div>
      </div>
    </section>
  );
}
