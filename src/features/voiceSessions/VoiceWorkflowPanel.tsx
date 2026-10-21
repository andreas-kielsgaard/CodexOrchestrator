import { useEffect, useRef, useState } from 'react';
import { JevQueryInspector } from '../agentProviders/jev/JevQueryInspector';
import type { JevQueryClient } from '../../application/agentSessions/jevQueries';
import { voiceQuestionSubject } from './voiceJevQueryPresentation';
import { NodeNotificationEditor } from '../workflowGraph/NodeNotificationEditor';
import type { VoicePrompts, VoiceSettings, VoiceView } from '../../application/voiceSessions';
import {
  ExtensionWorkflowCanvas,
  type ExtensionWorkflowSelection,
} from '../extensionWorkflows/ExtensionWorkflowCanvas';
import { VoiceBehaviorEditor } from './VoiceBehaviorEditor';
import { voicePromptName, type VoiceBehaviorTarget } from './voiceWorkflowPresentation';
import { VoiceRouteInspector, type VoiceActivityRecord } from './VoiceRouteInspector';
import './voiceWorkflow.css';

export function VoiceWorkflowPanel({
  view,
  settings,
  busy,
  onEdit,
  onOpenSession,
  onOpenProfile,
  queryClient,
  active,
  routeRequest,
  onShowActivity,
}: {
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit(change: Partial<VoiceSettings>): void;
  onOpenSession(id: string, invocationId?: string): void;
  onOpenProfile?(id: string): void;
  queryClient?: JevQueryClient;
  active: boolean;
  routeRequest: number;
  onShowActivity(): void;
}) {
  const [selection, setSelection] = useState<ExtensionWorkflowSelection>(null);
  const [mode, setMode] = useState<'map' | 'editor' | 'queries'>('map');
  const [editor, setEditor] = useState<VoiceBehaviorTarget>({ kind: 'prompt', key: 'tools' });
  const [pane, setPane] = useState<'structure' | 'routes'>('structure');
  const [selectedActivity, setSelectedActivity] = useState<string | null>(null);
  const [queriesVisited, setQueriesVisited] = useState(false);
  const [returnToEvaluation, setReturnToEvaluation] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const returnScroll = useRef(0);
  const activity = view.state.conversation?.activity ?? [];
  useEffect(() => {
    setSelectedActivity(null);
    setReturnToEvaluation(false);
  }, [view.state.conversation?.id]);
  useEffect(() => {
    if (routeRequest) {
      setPane('routes');
      setMode('map');
      setSelectedActivity(
        (current) => current ?? view.state.conversation?.activity?.at(-1)?.id ?? null,
      );
    }
    // Navigation request, not a reset on status polling.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [routeRequest]);
  const leaveMap = (next: 'editor' | 'queries') => {
    returnFocus.current = document.activeElement as HTMLElement;
    returnScroll.current = panel.current?.closest('main')?.scrollTop ?? 0;
    setMode(next);
    if (next === 'queries') setQueriesVisited(true);
    requestAnimationFrame(() =>
      panel.current?.querySelector<HTMLElement>('[data-workspace-heading]')?.focus(),
    );
  };
  const backToMap = () => {
    setMode('map');
    requestAnimationFrame(() => {
      const main = panel.current?.closest('main');
      if (main) main.scrollTop = returnScroll.current;
      returnFocus.current?.focus({ preventScroll: true });
    });
  };
  const editBehavior = (target: VoiceBehaviorTarget) => {
    setEditor(target);
    leaveMap('editor');
  };
  const inspect = (next: ExtensionWorkflowSelection) => {
    setSelection(next);
    setPane('structure');
    setReturnToEvaluation(false);
  };
  const selectRecord = (record: VoiceActivityRecord) => {
    setSelectedActivity(record.id);
    setPane('routes');
    setMode('map');
    setSelection(record.route ? { kind: 'node', id: record.route.sourceNodeId } : null);
  };
  const definition = view.workflow;
  if (!definition)
    return (
      <p>
        Workflow metadata is unavailable.{' '}
        <button onClick={onShowActivity}>Open conversation activity</button>
      </p>
    );
  const state = view.state;
  const conversation = state.conversation;
  const node = definition.nodes.find(
    (node) => selection?.kind === 'node' && node.id === selection.id,
  );
  const draftReaction = settings.workflow?.directReactions ?? true;
  const routeChanged = draftReaction !== (state.settings.workflow?.directReactions ?? true);
  const draftDefinition = {
    ...definition,
    connections: definition.connections.map((edge) =>
      edge.id === 'buffer-output' ? { ...edge, enabled: draftReaction } : edge,
    ),
  };
  const displayedDefinition = pane === 'routes' ? definition : draftDefinition;
  const connection = displayedDefinition.connections.find(
    (edge) => selection?.kind === 'connection' && edge.id === selection.id,
  );
  const selectedRecord = activity.find((record) => record.id === selectedActivity);
  const name = (id: string) => definition.nodes.find((n) => n.id === id)?.name ?? id;
  const filterSession = definition.nodes.find((n) => n.id === 'filter')?.sessionIds[0];
  const statuses: Record<string, string> = {
    transcript: view.diagnostics?.pendingAudio
      ? 'Transcribing'
      : state.phoneConnected
        ? 'Phone connected'
        : 'Phone disconnected',
    filter: view.diagnostics?.input.last?.filterPending
      ? 'Evaluating'
      : view.diagnostics?.filterError
        ? 'Evaluation failed'
        : (view.diagnostics?.input.last?.relevance ?? 'Waiting for input'),
    buffer: `${conversation?.inputBuffer?.blocks.length ?? 0} blocks held${conversation?.inputBuffer?.complete ? ' · ready' : ''}`,
    workers: `${conversation?.delegations.length ?? 0} assignments`,
  };
  for (const stage of view.diagnostics?.stages ?? [])
    if (stage.role !== 'worker')
      statuses[stage.role] =
        `${stage.status ?? 'Idle'}${stage.queued ? ` · ${stage.queued} pending` : ''}`;
  const nodeRoutes = definition.connections.filter(
    (edge) => edge.source === node?.id || edge.destination === node?.id,
  );
  const promptKeys =
    node?.promptKeys.filter((key): key is keyof VoicePrompts => key in settings.prompts) ?? [];
  const locked = busy || state.enabled;
  return (
    <div className="voice-workflow" ref={panel}>
      {mode === 'map' && returnToEvaluation && (
        <nav className="voice-breadcrumb" aria-label="Evaluation route">
          <button
            onClick={() => {
              setSelection({ kind: 'node', id: 'filter' });
              setPane('structure');
              setMode('queries');
              setReturnToEvaluation(false);
            }}
          >
            ← Back to evaluation
          </button>
          <span>/ Released request</span>
        </nav>
      )}
      {mode !== 'map' && (
        <nav className="voice-breadcrumb" aria-label="Workflow location">
          <button onClick={backToMap}>← Back to {node?.name ?? 'workflow'}</button>
          <span>
            /{' '}
            {mode === 'queries'
              ? 'Evaluations'
              : editor.kind === 'prompt'
                ? voicePromptName(editor.key)
                : editor.kind === 'model'
                  ? 'Shared models'
                  : 'Routing preferences'}
          </span>
        </nav>
      )}
      <div hidden={mode !== 'map'}>
        <div className="voice-workflow-toolbar">
          <nav className="voice-tabs" aria-label="Workflow inspection">
            <button aria-pressed={pane === 'structure'} onClick={() => setPane('structure')}>
              Structure
            </button>
            <button
              aria-pressed={pane === 'routes'}
              onClick={() => {
                setPane('routes');
                if (!selectedActivity && activity.length) selectRecord(activity.at(-1)!);
              }}
            >
              Recorded routes
            </button>
          </nav>
          {pane === 'structure' && routeChanged && (
            <span className="voice-draft-label">Previewing draft route · not yet applied</span>
          )}
          {pane === 'routes' && (
            <span className="voice-muted">Delivery evidence on the saved map</span>
          )}
        </div>
        <div className="voice-workflow-body">
          <ExtensionWorkflowCanvas
            definition={displayedDefinition}
            selection={selection}
            onSelect={inspect}
            statuses={statuses}
            traceMode={pane === 'routes'}
            highlightedIds={pane === 'routes' ? selectedRecord?.route?.connectionIds : undefined}
          />
          <aside className="voice-panel voice-workflow-inspector" aria-label="Workflow inspector">
            {pane === 'routes' ? (
              <VoiceRouteInspector
                view={view}
                selectedId={selectedActivity}
                onSelect={selectRecord}
                onInspectNode={(id) => inspect({ kind: 'node', id })}
                onOpenSession={onOpenSession}
              />
            ) : (
              <>
                {!selection && (
                  <>
                    <h2>How Voice works</h2>
                    <p>
                      Speech is transcribed and evaluated. Relevant input is held until it can be
                      routed.
                    </p>
                    <p>
                      A request can trigger a quick receipt, delegated work and a direct response.
                      These paths can run alongside one another.
                    </p>
                    <p>
                      Digest observes the conversation and work, then asks Output to respond when
                      useful.
                    </p>
                    <h3>Explore a stage</h3>
                    <p>
                      Select a stage to inspect its purpose, incoming and outgoing routes, prompts
                      and dependencies.
                    </p>
                    <h3>What you can change</h3>
                    <p>
                      Prompts, shared tool instructions, model groups and the direct-reaction route
                      are editable here. Voice owns the stage structure; adding stages or
                      connections requires extending the integration.
                    </p>
                    <button onClick={() => editBehavior({ kind: 'prompt', key: 'tools' })}>
                      Inspect shared tools and integrations
                    </button>
                  </>
                )}
                {node && (
                  <>
                    <small>{node.kind.replaceAll('_', ' ')}</small>
                    <h2>{node.name}</h2>
                    <p>{node.description}</p>
                    <p className="voice-muted">{statuses[node.id] ?? 'No active invocation'}</p>
                    {node.id === 'buffer' && (
                      <button onClick={() => inspect({ kind: 'connection', id: 'buffer-output' })}>
                        Edit direct reaction route
                      </button>
                    )}
                    <details
                      key={node.id}
                      open={nodeRoutes.length <= 3}
                      aria-label="Stage connections"
                    >
                      <summary>
                        Inputs and outputs ·{' '}
                        {nodeRoutes.filter((edge) => edge.destination === node.id).length} incoming,{' '}
                        {nodeRoutes.filter((edge) => edge.source === node.id).length} outgoing
                      </summary>
                      {nodeRoutes.map((edge) => (
                        <button
                          className="voice-route-link"
                          key={edge.id}
                          onClick={() => inspect({ kind: 'connection', id: edge.id })}
                        >
                          <small>
                            {edge.source === node.id ? 'To' : 'From'}{' '}
                            {name(edge.source === node.id ? edge.destination : edge.source)}
                          </small>
                          <span>
                            {edge.name}
                            {edge.id === 'buffer-output' && !draftReaction
                              ? routeChanged
                                ? ' · disabled in draft'
                                : ' · disabled'
                              : ''}
                          </span>
                        </button>
                      ))}
                      {!nodeRoutes.length && <p>No connections described for this stage.</p>}
                    </details>
                    {!!promptKeys.length && (
                      <section>
                        <h3>Behavior</h3>
                        {promptKeys.map((key) => (
                          <button key={key} onClick={() => editBehavior({ kind: 'prompt', key })}>
                            {voicePromptName(key)}
                          </button>
                        ))}
                      </section>
                    )}
                    {(node.kind === 'agent' || node.kind === 'external_sessions') && (
                      <section>
                        <h3>Tools and shared settings</h3>
                        <button onClick={() => editBehavior({ kind: 'prompt', key: 'tools' })}>
                          Tools and integrations
                        </button>
                        <button
                          onClick={() =>
                            editBehavior({
                              kind: 'model',
                              group:
                                node.id === 'router' || node.id === 'workers'
                                  ? 'routing'
                                  : 'response',
                            })
                          }
                        >
                          {node.id === 'router' || node.id === 'workers' ? 'Routing' : 'Response'}{' '}
                          model group
                        </button>
                        {node.id === 'router' && (
                          <button onClick={() => editBehavior({ kind: 'preferences' })}>
                            Routing preferences
                          </button>
                        )}
                      </section>
                    )}
                    {node.id === 'filter' && (
                      <section>
                        <h3>Execution evidence</h3>
                        <button
                          disabled={!queryClient || !filterSession}
                          onClick={() => leaveMap('queries')}
                        >
                          Inspect evaluations
                        </button>
                        {!filterSession && (
                          <p className="voice-muted">
                            Evaluations become available after a conversation has been started.
                          </p>
                        )}
                      </section>
                    )}
                    {!!node.sessionIds.length && (
                      <details>
                        <summary>Agent sessions · {node.sessionIds.length}</summary>
                        {[...new Set(node.sessionIds)].map((id) => (
                          <button key={id} onClick={() => onOpenSession(id)}>
                            Open {node.name} session
                          </button>
                        ))}
                      </details>
                    )}
                    {['transcript', 'filter', 'buffer'].includes(node.id) && state.inputPolicy && (
                      <details>
                        <summary>Input timing and context</summary>
                        <dl>
                          <dt>Evaluate after</dt>
                          <dd>
                            {state.inputPolicy.sentencesPerQuery} sentences or{' '}
                            {state.inputPolicy.pauseSeconds}s pause
                          </dd>
                          <dt>Context window</dt>
                          <dd>{state.inputPolicy.contextSeconds}s</dd>
                          <dt>Follow-up window</dt>
                          <dd>{state.inputPolicy.followupSeconds}s</dd>
                        </dl>
                      </details>
                    )}
                    {node.kind === 'agent' && (
                      <details>
                        <summary>Turn notifications</summary>
                        <NodeNotificationEditor
                          disabled={locked}
                          value={{
                            turnCompletionSound:
                              settings.workflow?.notificationSounds?.[node.id] ?? false,
                          }}
                          onChange={(notifications) =>
                            onEdit({
                              workflow: {
                                ...settings.workflow,
                                directReactions: draftReaction,
                                notificationSounds: {
                                  ...settings.workflow?.notificationSounds,
                                  [node.id]: notifications.turnCompletionSound,
                                },
                              },
                            })
                          }
                        />
                      </details>
                    )}
                    {node.id === 'workers' && (
                      <p className="voice-muted">
                        Assignments run in separate Agent Sessions. Stopping the connection does not
                        cancel them.
                      </p>
                    )}
                  </>
                )}
                {connection && (
                  <>
                    <h2>{connection.name}</h2>
                    <div className="voice-edge-endpoints">
                      <button onClick={() => inspect({ kind: 'node', id: connection.source })}>
                        {name(connection.source)}
                      </button>
                      <span>→</span>
                      <button onClick={() => inspect({ kind: 'node', id: connection.destination })}>
                        {name(connection.destination)}
                      </button>
                    </div>
                    <h3>When</h3>
                    <p>{connection.condition}</p>
                    <h3>What passes through</h3>
                    <p>{connection.payload}</p>
                    {connection.id === 'buffer-output' ? (
                      <>
                        <label className="voice-check">
                          <input
                            type="checkbox"
                            disabled={locked}
                            checked={draftReaction}
                            onChange={(e) =>
                              onEdit({
                                workflow: {
                                  ...settings.workflow,
                                  directReactions: e.target.checked,
                                },
                              })
                            }
                          />
                          Route reaction-only requests directly to Output
                        </label>
                        <p className="voice-muted">
                          {routeChanged
                            ? 'Draft route change. Save to apply it.'
                            : 'Matches the saved route.'}{' '}
                          {state.enabled ? 'Stop the connection to edit.' : ''}
                        </p>
                        <button onClick={() => editBehavior({ kind: 'prompt', key: 'reaction' })}>
                          Inspect reaction criteria
                        </button>
                      </>
                    ) : (
                      <p className="voice-muted">
                        Voice owns this connection. Inspect its endpoints to find the prompts and
                        dependencies that govern their behavior.
                      </p>
                    )}
                    <details>
                      <summary>Event identifier</summary>
                      <code>{connection.event}</code>
                    </details>
                  </>
                )}
              </>
            )}
          </aside>
        </div>
      </div>
      {mode === 'editor' && (
        <div>
          <h2 className="voice-workspace-heading" tabIndex={-1} data-workspace-heading>
            Behavior settings
          </h2>
          <VoiceBehaviorEditor
            target={editor}
            view={view}
            settings={settings}
            busy={busy}
            onEdit={onEdit}
            onOpenProfile={onOpenProfile}
          />
        </div>
      )}
      {queriesVisited && queryClient && filterSession && (
        <div hidden={mode !== 'queries'}>
          <h2 className="voice-workspace-heading" tabIndex={-1} data-workspace-heading>
            Evaluation history
          </h2>
          <JevQueryInspector
            client={queryClient}
            sessionId={filterSession}
            active={active && mode === 'queries'}
            applications={conversation?.jevApplications}
            questionSubject={voiceQuestionSubject}
            onShowRelease={(requestId) => {
              const record = activity.find(
                (a) => a.kind === 'accepted_input' && a.data.requestId === requestId,
              );
              if (record) {
                selectRecord(record);
                setReturnToEvaluation(true);
              }
            }}
          />
        </div>
      )}
    </div>
  );
}
