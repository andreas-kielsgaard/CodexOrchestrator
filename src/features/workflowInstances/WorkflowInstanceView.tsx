import { RefreshCw, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { AgentSessionClient } from '../../application/agentSessions';
import type { AgentSessionProfileClient } from '../../application/agentSessions';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type {
  SessionEventQueryClient,
  SessionEventResultDto,
} from '../../application/sessionEvents';
import type {
  WorkflowInstanceClient,
  WorkflowInstanceDetails,
} from '../../application/workflowInstances';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { ProfiledSessionPane } from '../agentSessions/ProfiledSessionPane';
import { NodeProfileInspector, type AgentIdentityOption } from '../executionConfiguration';
import { AgentIdentityBadge } from '../identities';
import { EventGroupInspector } from '../sessionEvents';
import {
  WorkflowGraphConnections,
  WorkflowGraphEmpty,
  WorkflowGraphNodeCard,
  WorkflowGraphSurface,
  workflowGraphBounds,
} from '../workflowGraph';
import {
  attemptStatus,
  attemptsForConnection,
  attemptsForNode,
  attemptsWithoutElement,
  connectionById,
  instanceGraphConnections,
  instanceGraphNodes,
  nodeById,
  sessionsForNode,
  type WorkflowInstanceSelection,
} from './workflowInstancePresentation';
import './workflowInstanceView.css';

export interface WorkflowInstanceViewProps {
  readonly sessionFocus?: { readonly nodeId: string; readonly sessionId: string };
  readonly instanceId: string;
  readonly client: WorkflowInstanceClient;
  readonly capabilityProfiles?: ReadonlyMap<string, CapabilityProfileDto>;
  readonly identities?: readonly AgentIdentityOption[];
  readonly sessionClient?: AgentSessionClient;
  readonly profileClient?: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
}

export function WorkflowInstanceView({
  instanceId,
  sessionFocus,
  client,
  capabilityProfiles = new Map(),
  identities = [],
  sessionClient,
  profileClient,
  queryClient,
}: WorkflowInstanceViewProps) {
  const [details, setDetails] = useState<WorkflowInstanceDetails | null>(null);
  const [selection, setSelection] = useState<WorkflowInstanceSelection>(
    sessionFocus
      ? { kind: 'session', id: sessionFocus.sessionId, nodeId: sessionFocus.nodeId }
      : null,
  );
  const [requestText, setRequestText] = useState('');
  const [eventResult, setEventResult] = useState<SessionEventResultDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
      setError(null);
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
  }, [client, instanceId, load]);

  if (!details) {
    return (
      <section className="workflow-instance-view is-loading">
        <p>{error ?? 'Loading instance…'}</p>
        <button type="button" onClick={() => void load()}>
          Refresh
        </button>
      </section>
    );
  }

  const graphNodes = instanceGraphNodes(details);
  const graphConnections = instanceGraphConnections(details);
  const { width, height } = workflowGraphBounds(graphNodes);
  const selectedNode = selection?.kind === 'node' ? nodeById(details, selection.id) : undefined;
  const selectedConnection =
    selection?.kind === 'connection' ? connectionById(details, selection.id) : undefined;
  const selectedConnectionAttempts = selectedConnection
    ? attemptsForConnection(details.attempts, selectedConnection.connectionId)
    : [];
  const unassignedAttempts = attemptsWithoutElement(details.attempts);

  const loadAttempt = async (
    eventGroup: NonNullable<(typeof details.attempts)[number]['eventGroup']>,
  ) => {
    if (!queryClient) return;
    setError(null);
    try {
      const value = await queryClient.loadRecordedEvent(eventGroup);
      if (mounted.current) setEventResult(value);
    } catch (cause) {
      if (mounted.current) setError(String(cause));
    }
  };

  return (
    <section className="workflow-instance-view" data-testid="workflow-instance-view">
      <header className="workflow-instance-view__header">
        <div>
          <p>Workflow instance · pinned recipe v{details.instance.recipe.revision}</p>
          <h1>{details.instance.name}</h1>
          <span>{details.instance.recipe.name}</span>
        </div>
        <div>
          <small title={details.instance.target.worktree.path}>
            {details.instance.target.worktree.path}
          </small>
          <button type="button" onClick={() => void load()}>
            <RefreshCw size={15} aria-hidden="true" /> Refresh
          </button>
        </div>
      </header>
      {error ? (
        <p className="workflow-instance-view__error" role="alert">
          {error}
        </p>
      ) : null}
      <div className="workflow-instance-view__body">
        <WorkflowGraphSurface
          width={width}
          height={height}
          aria-label="Workflow instance flow"
          role="region"
          tabIndex={0}
        >
          <WorkflowGraphConnections
            nodes={graphNodes}
            connections={graphConnections}
            selectedId={selection?.kind === 'connection' ? selection.id : null}
            highlightedIds={graphConnections
              .filter(
                (connection) => attemptsForConnection(details.attempts, connection.id).length > 0,
              )
              .map((connection) => connection.id)}
            labelForConnection={(connection) => {
              const count = attemptsForConnection(details.attempts, connection.id).length;
              return count ? `${connection.name} · ${count}` : connection.name;
            }}
            ariaLabelForConnection={(connection) => `Open ${connection.name} activity`}
            onActivate={(connection) => {
              setEventResult(null);
              setSelection({ kind: 'connection', id: connection.id });
            }}
          />
          {graphNodes.map((graphNode) => {
            const node = nodeById(details, graphNode.id)!;
            const nodeSessions = sessionsForNode(details.sessions, graphNode.id);
            const nodeAttempts = attemptsForNode(details.attempts, graphNode.id);
            const identity = identities.find((candidate) => candidate.id === node.agentIdentityId);
            return (
              <WorkflowGraphNodeCard
                key={graphNode.id}
                node={graphNode}
                selected={
                  (selection?.kind === 'node' && selection.id === graphNode.id) ||
                  (selection?.kind === 'session' && selection.nodeId === graphNode.id)
                }
                aria-label={`Open ${graphNode.name}`}
                onClick={(event) => {
                  event.stopPropagation();
                  setEventResult(null);
                  setRequestText('');
                  setSelection({ kind: 'node', id: graphNode.id });
                }}
              >
                <span className="workflow-node__badges">
                  {graphNode.starting ? <small>Start</small> : null}
                  {identity ? <AgentIdentityBadge identity={identity} compact /> : null}
                  {nodeSessions.some((entry) => entry.running) ? (
                    <small className="is-running">Running</small>
                  ) : null}
                </span>
                <strong>{graphNode.name}</strong>
                <span>
                  {nodeSessions.length
                    ? `${nodeSessions.length} Session${nodeSessions.length === 1 ? '' : 's'}`
                    : 'No Sessions yet'}
                  {nodeAttempts.length
                    ? ` · ${nodeAttempts.length} event${nodeAttempts.length === 1 ? '' : 's'}`
                    : ''}
                </span>
              </WorkflowGraphNodeCard>
            );
          })}
          {!graphNodes.length ? (
            <WorkflowGraphEmpty>
              <strong>No nodes in this instance</strong>
              <span>The pinned recipe is empty.</span>
            </WorkflowGraphEmpty>
          ) : null}
        </WorkflowGraphSurface>

        {selection ? (
          <aside
            className="workflow-instance-view__inspector"
            aria-label="Selected runtime element"
          >
            <button
              type="button"
              className="workflow-instance-view__close"
              aria-label="Close details"
              onClick={() => {
                setSelection(null);
                setEventResult(null);
              }}
            >
              <X size={16} aria-hidden="true" /> Close
            </button>

            {selection.kind === 'session' ? (
              <SessionInspector
                sessionId={selection.id}
                nodeName={selection.nodeId ? nodeById(details, selection.nodeId)?.name : undefined}
                onBack={
                  selection.nodeId
                    ? () => setSelection({ kind: 'node', id: selection.nodeId! })
                    : undefined
                }
                sessionClient={sessionClient}
                profileClient={profileClient}
                queryClient={queryClient}
              />
            ) : selectedNode ? (
              <NodeInspector
                node={selectedNode}
                details={details}
                capabilityProfile={capabilityProfiles.get(selectedNode.capabilityProfileId)}
                identity={identities.find(
                  (candidate) => candidate.id === selectedNode.agentIdentityId,
                )}
                attempts={attemptsForNode(details.attempts, selectedNode.nodeId)}
                eventResult={eventResult}
                canLoadEvents={Boolean(queryClient)}
                requestText={requestText}
                busy={busy}
                onRequestText={setRequestText}
                onOpenSession={(sessionId) =>
                  setSelection({ kind: 'session', id: sessionId, nodeId: selectedNode.nodeId })
                }
                onLoadAttempt={loadAttempt}
                onSend={async () => {
                  if (busy || !requestText.trim()) return;
                  setBusy(true);
                  setError(null);
                  try {
                    const result = await client.messageNode({
                      instanceId,
                      recipeId: details.instance.recipe.recipeId,
                      nodeId: selectedNode.nodeId,
                      text: requestText,
                    });
                    if (!mounted.current) return;
                    setRequestText('');
                    setEventResult(result);
                    await load();
                    const sessionId = result.deliveries[0]?.targetSession.id;
                    if (sessionId && mounted.current) {
                      setSelection({ kind: 'session', id: sessionId, nodeId: selectedNode.nodeId });
                    }
                  } catch (cause) {
                    if (mounted.current) setError(String(cause));
                  } finally {
                    if (mounted.current) setBusy(false);
                  }
                }}
              />
            ) : selectedConnection ? (
              <ConnectionInspector
                name={selectedConnection.name}
                sourceName={nodeById(details, selectedConnection.sourceNodeId)?.name}
                destinationName={nodeById(details, selectedConnection.destinationNodeId)?.name}
                attempts={selectedConnectionAttempts}
                eventResult={eventResult}
                canLoadEvents={Boolean(queryClient)}
                onLoadAttempt={loadAttempt}
              />
            ) : null}
          </aside>
        ) : null}
      </div>
      {unassignedAttempts.length ? (
        <CollapsibleSection
          title={`Older activity (${unassignedAttempts.length})`}
          description="These records predate node and connection ownership."
          defaultExpanded={false}
          className="workflow-instance-view__older-activity"
        >
          {unassignedAttempts.map((attempt) => (
            <span key={attempt.id}>
              {attemptStatus(attempt)} · {attempt.createdAt}
            </span>
          ))}
        </CollapsibleSection>
      ) : null}
    </section>
  );
}

function NodeInspector({
  node,
  details,
  capabilityProfile,
  identity,
  attempts,
  eventResult,
  canLoadEvents,
  requestText,
  busy,
  onRequestText,
  onOpenSession,
  onLoadAttempt,
  onSend,
}: {
  readonly node: WorkflowInstanceDetails['instance']['recipe']['nodes'][number];
  readonly details: WorkflowInstanceDetails;
  readonly capabilityProfile?: CapabilityProfileDto;
  readonly identity?: AgentIdentityOption;
  readonly attempts: readonly WorkflowInstanceDetails['attempts'][number][];
  readonly eventResult: SessionEventResultDto | null;
  readonly canLoadEvents: boolean;
  readonly requestText: string;
  readonly busy: boolean;
  onRequestText(value: string): void;
  onOpenSession(sessionId: string): void;
  onLoadAttempt(
    eventGroup: NonNullable<WorkflowInstanceDetails['attempts'][number]['eventGroup']>,
  ): void;
  onSend(): void;
}) {
  const sessions = sessionsForNode(details.sessions, node.nodeId);
  return (
    <div className="workflow-instance-view__node-inspector">
      <header>
        <div>
          <p>Workflow node</p>
          <h2>{node.name}</h2>
        </div>
        {identity ? (
          <AgentIdentityBadge identity={identity} secondaryLabel="Agent identity" />
        ) : null}
      </header>
      <form
        className="workflow-instance-view__request"
        onSubmit={(event) => {
          event.preventDefault();
          onSend();
        }}
      >
        <label>
          Message this node
          <textarea
            rows={4}
            value={requestText}
            onChange={(event) => onRequestText(event.currentTarget.value)}
          />
        </label>
        <button type="submit" disabled={busy || !requestText.trim()}>
          {busy ? 'Sending…' : 'Send request'}
        </button>
        <small>A Session is created here if no target Session is available.</small>
      </form>
      <CollapsibleSection
        title={`Sessions (${sessions.length})`}
        className="workflow-instance-view__section"
      >
        {sessions.length ? (
          sessions.map((entry) => (
            <button
              key={entry.session.id}
              type="button"
              className="workflow-instance-view__session-link"
              onClick={() => onOpenSession(entry.session.id)}
            >
              <strong>{entry.running ? 'Running' : 'Idle'}</strong>
              <span>{entry.session.id}</span>
            </button>
          ))
        ) : (
          <p>No Sessions have been created at this node.</p>
        )}
      </CollapsibleSection>
      <CollapsibleSection
        title={`Activity (${attempts.length})`}
        defaultExpanded={false}
        className="workflow-instance-view__section"
      >
        <AttemptList
          attempts={attempts}
          canLoadEvents={canLoadEvents}
          onLoadAttempt={onLoadAttempt}
        />
        {eventResult ? (
          <EventGroupInspector group={eventResult.group} deliveries={eventResult.deliveries} />
        ) : null}
      </CollapsibleSection>
      <CollapsibleSection
        title="Initial prompt"
        description="Used only when this node creates a Session."
        defaultExpanded={false}
        className="workflow-instance-view__section"
      >
        <pre className="workflow-instance-view__prompt">
          {node.initialPrompt || 'No initial prompt.'}
        </pre>
      </CollapsibleSection>
      <NodeProfileInspector
        nodeProfile={node.nodeProfile}
        capabilityProfile={capabilityProfile}
        capabilityProfileId={node.capabilityProfileId}
      />
    </div>
  );
}

function SessionInspector({
  sessionId,
  nodeName,
  onBack,
  sessionClient,
  profileClient,
  queryClient,
}: {
  readonly sessionId: string;
  readonly nodeName?: string;
  readonly onBack?: () => void;
  readonly sessionClient?: AgentSessionClient;
  readonly profileClient?: AgentSessionProfileClient;
  readonly queryClient?: SessionEventQueryClient;
}) {
  return (
    <div className="workflow-instance-view__session-inspector">
      <header>
        <div>
          <p>{nodeName ?? 'Workflow node'}</p>
          <h2>Agent Session</h2>
        </div>
        {onBack ? (
          <button type="button" onClick={onBack}>
            Back to node
          </button>
        ) : null}
      </header>
      {sessionClient && profileClient ? (
        <ProfiledSessionPane
          sessionId={sessionId}
          client={sessionClient}
          profileClient={profileClient}
          queryClient={queryClient}
        />
      ) : (
        <p>Session details are not available in this host.</p>
      )}
    </div>
  );
}

function ConnectionInspector({
  name,
  sourceName,
  destinationName,
  attempts,
  eventResult,
  canLoadEvents,
  onLoadAttempt,
}: {
  readonly name: string;
  readonly sourceName?: string;
  readonly destinationName?: string;
  readonly attempts: readonly WorkflowInstanceDetails['attempts'][number][];
  readonly eventResult: SessionEventResultDto | null;
  readonly canLoadEvents: boolean;
  onLoadAttempt(
    eventGroup: NonNullable<WorkflowInstanceDetails['attempts'][number]['eventGroup']>,
  ): void;
}) {
  return (
    <div className="workflow-instance-view__connection-inspector">
      <header>
        <p>Workflow connection</p>
        <h2>{name}</h2>
        <span>
          {sourceName ?? 'Unknown'} → {destinationName ?? 'Unknown'}
        </span>
      </header>
      <div className="workflow-instance-view__attempts">
        <h3>Activity ({attempts.length})</h3>
        <AttemptList
          attempts={attempts}
          canLoadEvents={canLoadEvents}
          onLoadAttempt={onLoadAttempt}
          emptyMessage="No activity has used this connection."
        />
      </div>
      {eventResult ? (
        <EventGroupInspector group={eventResult.group} deliveries={eventResult.deliveries} />
      ) : null}
    </div>
  );
}

function AttemptList({
  attempts,
  canLoadEvents,
  onLoadAttempt,
  emptyMessage = 'No activity has been recorded here.',
}: {
  readonly attempts: readonly WorkflowInstanceDetails['attempts'][number][];
  readonly canLoadEvents: boolean;
  readonly emptyMessage?: string;
  onLoadAttempt(
    eventGroup: NonNullable<WorkflowInstanceDetails['attempts'][number]['eventGroup']>,
  ): void;
}) {
  if (!attempts.length) return <p>{emptyMessage}</p>;
  return attempts.map((attempt) => (
    <button
      key={attempt.id}
      type="button"
      className="workflow-instance-view__attempt"
      disabled={!attempt.eventGroup || !canLoadEvents}
      onClick={() => attempt.eventGroup && onLoadAttempt(attempt.eventGroup)}
    >
      <strong>{attemptStatus(attempt)}</strong>
      <span>{new Date(attempt.createdAt).toLocaleString()}</span>
      {attempt.error ? <small>{attempt.error}</small> : null}
    </button>
  ));
}
