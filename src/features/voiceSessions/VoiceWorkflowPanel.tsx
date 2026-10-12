import { useState } from 'react';
import { JevQueryInspector } from '../agentProviders/jev/JevQueryInspector';
import { voiceQuestionSubject } from './voiceJevQueryPresentation';
import { NodeNotificationEditor } from '../workflowGraph/NodeNotificationEditor';
import type { VoicePrompts, VoiceSettings, VoiceView } from '../../application/voiceSessions';
import {
  ExtensionWorkflowCanvas,
  type ExtensionWorkflowSelection,
} from '../extensionWorkflows/ExtensionWorkflowCanvas';
import './voiceWorkflow.css';

function promptKey(key: string): key is keyof VoicePrompts {
  return [
    'relevance',
    'completion',
    'continuation',
    'reaction',
    'summary',
    'router',
    'digest',
    'output',
    'tools',
  ].includes(key);
}
export function VoiceWorkflowPanel({
  view,
  settings,
  busy,
  onEdit,
  onSave,
  onOpenSession,
  onStart,
  onStop,
  onSend,
  queryClient,
}: {
  view: VoiceView;
  settings: VoiceSettings;
  busy: boolean;
  onEdit: (change: Partial<VoiceSettings>) => void;
  onSave: () => void;
  onOpenSession: (id: string) => void;
  onStart: () => void;
  onStop: () => void;
  onSend: (text: string) => void;
  queryClient?: import('../../application/agentSessions/jevQueries').JevQueryClient;
}) {
  const [selection, setSelection] = useState<ExtensionWorkflowSelection>({
    kind: 'node',
    id: 'filter',
  });
  const [text, setText] = useState('');
  const [queryTab, setQueryTab] = useState<'queries' | 'configuration'>('queries');
  const [selectedActivity, setSelectedActivity] = useState<string | null>(null);
  const definition = view.workflow;
  if (!definition)
    return <p>Workflow metadata is unavailable. Open Activity for the conversation.</p>;
  const state = view.state;
  const conversation = state.conversation;
  const node = definition.nodes.find(
    (node) => selection.kind === 'node' && node.id === selection.id,
  );
  const connection = definition.connections.find(
    (edge) => selection.kind === 'connection' && edge.id === selection.id,
  );
  const siblings = connection
    ? definition.connections.filter(
        (edge) => edge.source === connection.source && edge.destination === connection.destination,
      )
    : [];
  const activity = conversation?.activity ?? [];
  const selectedRecord = activity.find((record) => record.id === selectedActivity);
  const saved = JSON.stringify(settings) === JSON.stringify(state.settings);
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
  for (const stage of view.diagnostics?.stages ?? []) {
    if (stage.role !== 'worker')
      statuses[stage.role] =
        `${stage.status ?? 'Idle'}${stage.queued ? ` · ${stage.queued} pending` : ''}`;
  }
  const inputPolicy = state.inputPolicy;
  return (
    <div className="voice-workflow">
      <section className="voice-runtime" aria-label="Associated voice runtime">
        <div>
          <strong>{definition.runtime.name}</strong>
          <small>{definition.runtime.services.join(' · ')}</small>
        </div>
        <span>{state.enabled ? 'Running' : 'Stopped'}</span>
        <span>{state.phoneConnected ? 'Phone connected' : 'Phone disconnected'}</span>
        <span>Microphone {view.diagnostics?.capture?.recording ? 'recording' : 'off'}</span>
        <button disabled={busy || state.enabled || !settings.profileId || !saved} onClick={onStart}>
          Start connection
        </button>
        <button disabled={busy || !state.enabled} onClick={onStop}>
          Stop connection
        </button>
      </section>
      <div className="voice-workflow-toolbar">
        <div>
          <h2>{definition.name}</h2>
          <small>Provided by Voice · Select a stage or connection</small>
        </div>
        <span>{saved ? 'Saved configuration' : 'Unsaved configuration'}</span>
        <button disabled={busy || state.enabled || saved} onClick={onSave}>
          Save workflow settings
        </button>
      </div>
      <div className="voice-workflow-body">
        <ExtensionWorkflowCanvas
          definition={definition}
          selection={selection}
          onSelect={setSelection}
          statuses={statuses}
          highlightedIds={selectedRecord?.route?.connectionIds}
        />
        <aside className="voice-panel voice-workflow-inspector" aria-label="Workflow inspector">
          {node && (
            <>
              <small>{node.kind.replaceAll('_', ' ')}</small>
              <h2>{node.name}</h2>
              <p>{node.description}</p>
              <p>{statuses[node.id] ?? 'No active invocation'}</p>
              {node.id === 'filter' && queryClient && (
                <nav className="voice-tabs" aria-label="JEV node views">
                  <button
                    aria-pressed={queryTab === 'queries'}
                    onClick={() => setQueryTab('queries')}
                  >
                    Queries
                  </button>
                  <button
                    aria-pressed={queryTab === 'configuration'}
                    onClick={() => setQueryTab('configuration')}
                  >
                    Configuration
                  </button>
                </nav>
              )}
              {node.id === 'filter' && queryClient && queryTab === 'queries' && (
                <p>
                  Query history appears below the graph. Each evaluation contains its own submitted
                  context, questions and responses.
                </p>
              )}
              <div hidden={node.id === 'filter' && !!queryClient && queryTab === 'queries'}>
                <NodeNotificationEditor
                  disabled={state.enabled || busy}
                  value={{
                    turnCompletionSound: settings.workflow?.notificationSounds?.[node.id] ?? false,
                  }}
                  onChange={(notifications) =>
                    onEdit({
                      workflow: {
                        ...settings.workflow,
                        directReactions: settings.workflow?.directReactions ?? true,
                        notificationSounds: {
                          ...settings.workflow?.notificationSounds,
                          [node.id]: notifications.turnCompletionSound,
                        },
                      },
                    })
                  }
                />
                {node.id === 'filter' && view.diagnostics?.input.last && (
                  <article>
                    <strong>Latest evaluated input</strong>
                    <p>{view.diagnostics.input.last.text}</p>
                    <p>
                      {view.diagnostics.input.last.relevance ?? 'Not evaluated'} ·{' '}
                      {view.diagnostics.input.last.completion ?? 'Completion not checked'}
                    </p>
                  </article>
                )}
                {[...new Set(node.sessionIds)].map((id) => (
                  <button key={id} onClick={() => onOpenSession(id)}>
                    Open {node.name} session
                  </button>
                ))}
                {!!node.promptKeys.length && (
                  <>
                    <p className="voice-muted">
                      Stop the connection to edit. Save applies to subsequent evaluations and agent
                      turns.
                    </p>
                    {node.promptKeys.filter(promptKey).map((key) => (
                      <div key={key}>
                        <label>
                          {key[0].toUpperCase() + key.slice(1)} prompt
                          <textarea
                            rows={key === 'router' || key === 'output' ? 12 : 6}
                            disabled={state.enabled || busy}
                            value={settings.prompts[key]}
                            onChange={(event) =>
                              onEdit({
                                prompts: { ...settings.prompts, [key]: event.target.value },
                              })
                            }
                          />
                        </label>
                        <button
                          disabled={state.enabled || busy}
                          onClick={() =>
                            onEdit({
                              prompts: { ...settings.prompts, [key]: view.defaultPrompts[key] },
                            })
                          }
                        >
                          Reset {key}
                        </button>
                      </div>
                    ))}
                  </>
                )}
                {node.kind === 'agent' && (
                  <p className="voice-muted">
                    {node.id === 'router' ? 'Routing model' : 'Response model'}:{' '}
                    {settings.models[node.id === 'router' ? 'routing' : 'response'].model}. Model
                    groups are edited in Setup and apply to new conversations.
                  </p>
                )}
                {(node.id === 'filter' || node.id === 'transcript' || node.id === 'buffer') &&
                  inputPolicy && (
                    <dl>
                      <dt>Query threshold</dt>
                      <dd>
                        {inputPolicy.sentencesPerQuery} sentences or {inputPolicy.pauseSeconds}s
                        pause
                      </dd>
                      <dt>Context window</dt>
                      <dd>{inputPolicy.contextSeconds}s</dd>
                      <dt>Follow-up window</dt>
                      <dd>{inputPolicy.followupSeconds}s</dd>
                    </dl>
                  )}
                {node.id === 'buffer' && (
                  <label>
                    <input
                      type="checkbox"
                      disabled={state.enabled || busy}
                      checked={settings.workflow?.directReactions ?? true}
                      onChange={(event) =>
                        onEdit({
                          workflow: { ...settings.workflow, directReactions: event.target.checked },
                        })
                      }
                    />
                    Route reaction-only requests directly to Output
                  </label>
                )}
              </div>
              {node.id === 'workers' && (
                <p>
                  Assignments run in separate Agent Sessions. Stopping the voice connection does not
                  cancel them.
                </p>
              )}
            </>
          )}
          {connection && (
            <>
              <h2>{connection.name}</h2>
              {siblings.length > 1 && (
                <label>
                  Connection
                  <select
                    value={connection.id}
                    onChange={(event) =>
                      setSelection({ kind: 'connection', id: event.target.value })
                    }
                  >
                    {siblings.map((edge) => (
                      <option key={edge.id} value={edge.id}>
                        {edge.name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              <p>
                {connection.source} → {connection.destination}
              </p>
              <dl>
                <dt>Event</dt>
                <dd>{connection.event.replaceAll('_', ' ')}</dd>
                <dt>Condition</dt>
                <dd>{connection.condition}</dd>
                <dt>Payload</dt>
                <dd>{connection.payload}</dd>
              </dl>
              <p>
                {connection.enabled
                  ? 'Enabled in saved configuration'
                  : 'Disabled in saved configuration'}
              </p>
              {connection.id === 'buffer-output' && (
                <button onClick={() => setSelection({ kind: 'node', id: 'buffer' })}>
                  Edit reaction route
                </button>
              )}
              <p className="voice-muted">
                Voice owns this connection. Inspect its condition here; prompts determine semantic
                decisions.
              </p>
            </>
          )}
        </aside>
      </div>
      {node?.id === 'filter' && queryTab === 'queries' && queryClient && node.sessionIds[0] && (
        <JevQueryInspector
          key={node.sessionIds[0]}
          client={queryClient}
          sessionId={node.sessionIds[0]}
          applications={conversation?.jevApplications}
          questionSubject={voiceQuestionSubject}
          onShowRelease={(requestId) => {
            const record = activity.find(
              (a) => a.kind === 'accepted_input' && a.data.requestId === requestId,
            );
            if (record) setSelectedActivity(record.id);
          }}
        />
      )}
      <section className="voice-panel voice-flow-activity" aria-label="Workflow activity">
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onSend(text);
          }}
        >
          <label>
            Try a text request
            <input
              value={text}
              onChange={(event) => setText(event.target.value)}
              placeholder="Describe what you want naturally"
            />
          </label>
          <button disabled={busy || !state.enabled || !text.trim()}>Evaluate request</button>
        </form>
        <p className="voice-muted">
          Text goes through the real JEV and voice pipeline. Work routing may delegate tasks.
        </p>
        <h3>Recent routes</h3>
        {!activity.length && <p>Accepted requests and agent observations will appear here.</p>}
        {activity
          .slice(-12)
          .reverse()
          .map((record) => {
            const acceptedText = Array.isArray(record.data.blocks)
              ? record.data.blocks.map((block: { text?: string }) => block.text ?? '').join(' ')
              : undefined;
            const text = [
              record.data.text,
              record.data.reason,
              record.data.summary,
              record.data.finalReply,
              acceptedText,
            ].find((value) => typeof value === 'string');
            return (
              <article key={record.id}>
                <button
                  aria-pressed={selectedActivity === record.id}
                  onClick={() => setSelectedActivity(record.id)}
                >
                  {new Date(record.createdAt).toLocaleTimeString()} ·{' '}
                  {record.kind.replaceAll('_', ' ')}
                </button>
                {typeof text === 'string' && <p>{text.slice(0, 300)}</p>}
                {record.route ? (
                  <>
                    <p>
                      {record.route.sourceNodeId} →{' '}
                      {record.route.destinations
                        .map((destination) => destination.nodeId)
                        .join(', ') || 'No routed recipient'}
                    </p>
                    {record.route.destinations.map((destination) => {
                      const receipt = record.deliveries?.find(
                        (delivery) => delivery.sessionId === destination.sessionId,
                      )?.receipt;
                      return (
                        <button
                          key={destination.sessionId}
                          onClick={() => onOpenSession(destination.sessionId)}
                        >
                          {destination.nodeId} ·{' '}
                          {receipt
                            ? receipt.inputId
                              ? receipt.state
                              : 'Launch queued'
                            : 'Awaiting delivery receipt'}
                        </button>
                      );
                    })}
                  </>
                ) : (
                  <p className="voice-muted">Earlier activity · route evidence unavailable</p>
                )}
                <details>
                  <summary>Source and delivery references</summary>
                  <pre>{JSON.stringify(record, null, 2)}</pre>
                </details>
              </article>
            );
          })}
      </section>
    </div>
  );
}
