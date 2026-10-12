import { useEffect, useState } from 'react';
import type { VoiceSessionClient, VoiceSettings, VoiceView } from '../../application/voiceSessions';
import { VoiceSetupPanel } from './VoiceSetupPanel';
import { VoiceWorkflowPanel } from './VoiceWorkflowPanel';
import './voiceSessions.css';

export function VoiceSessionsScreen({
  client,
  onOpenSession,
  queryClient,
}: {
  client: VoiceSessionClient;
  onOpenSession: (id: string) => void;
  queryClient?: import('../../application/agentSessions/jevQueries').JevQueryClient;
}) {
  const [view, setView] = useState<VoiceView>();
  const [settings, setSettings] = useState<VoiceSettings>();
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [text, setText] = useState('');
  const [tab, setTab] = useState<'workflow' | 'activity' | 'setup'>('workflow');
  useEffect(() => {
    let alive = true;
    const read = () =>
      client
        .read()
        .then((result) => {
          if (!alive) return;
          setView(result);
          setSettings((current) => current ?? result.state.settings);
        })
        .catch((error: unknown) => {
          if (alive) setError(String(error));
        });
    void read();
    const timer = setInterval(() => void read(), 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [client]);
  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError('');
    try {
      await action();
      setView(await client.read());
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  };
  if (!view || !settings)
    return (
      <main className="voice-screen">
        <h1>Voice extension</h1>
        <p>{error || 'Loading conversation…'}</p>
      </main>
    );
  const state = view.state;
  const conversation = state.conversation;
  const captureFresh =
    !!view.diagnostics?.capture &&
    Date.now() - Date.parse(view.diagnostics.capture.updatedAt) < 5000;
  const edit = (change: Partial<VoiceSettings>) => setSettings({ ...settings, ...change });
  const saveSettings = async () => {
    const submitted = settings;
    const saved = await client.configure(submitted);
    setSettings((current) => (current === submitted ? saved.state.settings : current));
  };
  return (
    <main className="voice-screen">
      <header>
        <div>
          <h1>Voice extension</h1>
          <p>Voice interaction · Extension-owned workflow</p>
        </div>
        <span className={`voice-status ${state.phoneConnected ? 'connected' : ''}`}>
          {state.phoneConnected
            ? 'Phone connected'
            : state.enabled
              ? 'Waiting for phone'
              : 'Stopped'}
        </span>
      </header>
      {(error || state.error) && (
        <p role="alert" className="voice-error">
          {error || state.error}
        </p>
      )}
      <nav className="voice-tabs" aria-label="Voice extension views">
        {(['workflow', 'activity', 'setup'] as const).map((item) => (
          <button key={item} aria-pressed={tab === item} onClick={() => setTab(item)}>
            {item[0].toUpperCase() + item.slice(1)}
          </button>
        ))}
      </nav>
      {tab === 'workflow' && (
        <VoiceWorkflowPanel
          view={view}
          settings={settings}
          busy={busy}
          onEdit={edit}
          onSave={() => void run(saveSettings)}
          onOpenSession={onOpenSession}
          onStart={() =>
            void run(async () => {
              await client.configure(settings);
              await client.start(false);
            })
          }
          onStop={() => void run(() => client.stop())}
          onSend={(input) => void run(() => client.sendText(input))}
          queryClient={queryClient}
        />
      )}
      <div className="voice-columns voice-section">
        {tab === 'activity' && (
          <section className="voice-panel">
            <h2>Conversation</h2>
            <p>
              {state.speechConfigured
                ? 'Local transcription ready · phone text-to-speech'
                : 'Local speech setup required: npm run setup:voice'}
            </p>
            {view.diagnostics && (
              <section className="voice-diagnostics" aria-label="Live voice status">
                <h3>Live status</h3>
                <p>
                  <strong>{view.diagnostics.input.title}</strong>
                  <br />
                  {view.diagnostics.input.detail}
                </p>
                {view.diagnostics.filterError && (
                  <p className="voice-error">
                    JEV: {view.diagnostics.filterError} · input retained for reevaluation
                  </p>
                )}
                <dl>
                  <dt>Microphone</dt>
                  <dd>
                    {!state.phoneConnected ? (
                      'Phone disconnected'
                    ) : !view.diagnostics.capture ? (
                      'Connected · waiting for capture status from the phone'
                    ) : (
                      <>
                        {view.diagnostics.capture.routedMicrophone} ·{' '}
                        {!captureFresh
                          ? 'capture update missing'
                          : view.diagnostics.capture.recording
                            ? 'recording'
                            : 'stopped'}
                        {view.diagnostics.capture.silenced && ' · blocked by Android'}
                        {view.diagnostics.capture.requestedMicrophone === 'headset' &&
                          !view.diagnostics.capture.external &&
                          ' · external requested, phone fallback'}
                        <br />
                        Input signal {(view.diagnostics.capture.level * 100).toFixed(1)}% ·{' '}
                        {view.diagnostics.capture.segmentsSent} packets sent
                        {view.diagnostics.capture.error && (
                          <p className="voice-error">{view.diagnostics.capture.error}</p>
                        )}
                      </>
                    )}
                  </dd>
                  <dt>Transcription</dt>
                  <dd>
                    {view.diagnostics.audio.received} audio packets received ·{' '}
                    {view.diagnostics.audio.transcribed} context windows decoded ·{' '}
                    {view.diagnostics.pendingAudio} queued
                    {view.diagnostics.audio.lastReceivedAt && (
                      <>
                        <br />
                        Last audio{' '}
                        {new Date(view.diagnostics.audio.lastReceivedAt).toLocaleTimeString()}
                      </>
                    )}
                    {view.diagnostics.audio.lastTranscript != null && (
                      <>
                        <br />
                        Last transcript:{' '}
                        {view.diagnostics.audio.lastTranscript || '(no speech recognized)'}
                      </>
                    )}
                    {view.diagnostics.audio.error && (
                      <p className="voice-error">{view.diagnostics.audio.error}</p>
                    )}
                  </dd>
                  <dt>JEV decision</dt>
                  <dd>
                    {view.diagnostics.input.last ? (
                      <>
                        {view.diagnostics.input.last.filterPending
                          ? 'Evaluating'
                          : view.diagnostics.input.last.relevance}{' '}
                        · {view.diagnostics.input.last.completion ?? 'completion not checked'}
                        <br />
                        {view.diagnostics.input.last.text}
                      </>
                    ) : (
                      'No input evaluated yet'
                    )}
                  </dd>
                </dl>
                <div className="voice-stage-status">
                  {view.diagnostics.stages.map((stage) => (
                    <div key={stage.sessionId}>
                      <button className="voice-link" onClick={() => onOpenSession(stage.sessionId)}>
                        {stage.role === 'worker' ? (stage.title ?? 'Worker') : stage.role} ·{' '}
                        {stage.status ?? 'not started'}
                        {stage.queued > 0 ? ` · ${stage.queued} input pending` : ''}
                      </button>
                      {stage.error && <p className="voice-error">{stage.error}</p>}
                    </div>
                  ))}
                </div>
                <small className="voice-muted">
                  Updated {new Date(view.diagnostics.updatedAt).toLocaleTimeString()}
                </small>
              </section>
            )}
            <div className="voice-actions">
              <button
                disabled={busy || state.enabled || !settings.profileId}
                onClick={() =>
                  void run(async () => {
                    await client.configure(settings);
                    await client.start(false);
                  })
                }
              >
                {conversation ? 'Resume connection' : 'Start conversation'}
              </button>
              <button
                disabled={busy || !state.enabled}
                onClick={() => void run(() => client.stop())}
              >
                Stop connection
              </button>
              {conversation && (
                <button
                  disabled={busy || state.enabled}
                  onClick={() =>
                    void run(async () => {
                      await client.configure(settings);
                      await client.start(true);
                    })
                  }
                >
                  New conversation
                </button>
              )}
            </div>
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void run(async () => {
                  await client.sendText(text);
                  setText('');
                });
              }}
            >
              <label>
                Text input
                <input
                  value={text}
                  onChange={(event) => setText(event.target.value)}
                  placeholder="Try a request before using the phone"
                />
              </label>
              <button disabled={busy || !state.enabled || !text.trim()}>Send</button>
            </form>
            {conversation && (
              <>
                <button
                  className="voice-link"
                  onClick={() => onOpenSession(conversation.mainSessionId)}
                >
                  Open routing session
                </button>
                {conversation.pipeline && (
                  <section>
                    <h3>Reasoning sessions</h3>
                    {Object.entries(conversation.pipeline).map(([role, id]) => (
                      <button key={role} className="voice-link" onClick={() => onOpenSession(id)}>
                        Open {role === 'filter' ? 'JEV prefilter' : role}
                      </button>
                    ))}
                    <p>{conversation.inputBuffer?.blocks.length ?? 0} accepted blocks held</p>
                    {!!conversation.inputBuffer?.blocks.length && (
                      <details open>
                        <summary>Accepted input held</summary>
                        {conversation.inputBuffer.blocks.map((input) => (
                          <p key={input.id}>{input.text}</p>
                        ))}
                        <p className="voice-muted">
                          {conversation.inputBuffer.complete
                            ? 'Ready · waiting for speech and queued processing'
                            : 'Waiting for the input to be complete'}
                        </p>
                      </details>
                    )}
                    <details>
                      <summary>Input decisions</summary>
                      {conversation.inputs.slice(-10).map((input) => (
                        <p key={input.id}>
                          {input.text} ·{' '}
                          {input.filterPending
                            ? 'Evaluating'
                            : (input.relevance ?? 'Earlier input')}{' '}
                          {input.completion ? '· ' + input.completion : ''}
                        </p>
                      ))}
                    </details>
                  </section>
                )}
                <h3>Ready to discuss</h3>
                {conversation.pendingTopics.length ? (
                  conversation.pendingTopics.map((topic) => (
                    <article key={topic.id}>
                      <strong>{topic.title}</strong>
                      <details>
                        <summary>Source material</summary>
                        <p>{topic.material}</p>
                      </details>
                    </article>
                  ))
                ) : (
                  <p className="voice-muted">
                    The agent will announce useful material when it is ready.
                  </p>
                )}
                {!!conversation.tasks?.length && (
                  <section aria-label="Voice tasks">
                    <h3>Tasks</h3>
                    {conversation.tasks.map((task) => {
                      const assignments = conversation.delegations.filter(
                        (a) => a.taskId === task.id,
                      );
                      const latest = assignments.at(-1);
                      const reported = [...assignments].reverse().find((a) => a.outcome);
                      return (
                        <article key={task.id}>
                          <strong>{task.title}</strong>
                          <p className="voice-muted">
                            Request {task.revision} · {latest?.runtimeStatus ?? 'pending'}
                            {latest?.delivery?.inputId && ` · amendment ${latest.delivery.state}`}
                          </p>
                          <p>{task.request}</p>
                          {latest && (
                            <button
                              className="voice-link"
                              onClick={() => onOpenSession(latest.sessionId)}
                            >
                              Open worker
                            </button>
                          )}
                          {reported?.outcome && (
                            <details>
                              <summary>
                                {reported.outcome.revision < task.revision
                                  ? 'Earlier outcome'
                                  : 'Reported outcome'}{' '}
                                · request {reported.outcome.revision}
                              </summary>
                              <p>{reported.outcome.summary}</p>
                            </details>
                          )}
                        </article>
                      );
                    })}
                  </section>
                )}
                {!!conversation.activity?.length && (
                  <details>
                    <summary>Recent voice activity</summary>
                    {conversation.activity
                      .slice(-15)
                      .reverse()
                      .map((activity) => {
                        const data = activity.data;
                        const text = [
                          data.text,
                          data.reason,
                          data.summary,
                          data.task,
                          data.finalReply,
                        ].find((v) => typeof v === 'string');
                        return (
                          <article key={activity.id}>
                            <small>
                              {new Date(activity.createdAt).toLocaleTimeString()} ·{' '}
                              {activity.kind.replaceAll('_', ' ')}
                            </small>
                            {typeof text === 'string' && <p>{text.slice(0, 600)}</p>}
                            <details>
                              <summary>Source record</summary>
                              <pre>{JSON.stringify(activity, null, 2)}</pre>
                            </details>
                          </article>
                        );
                      })}
                  </details>
                )}
                <h3>Assignments</h3>
                {conversation.delegations.length ? (
                  conversation.delegations.map((work, i) => (
                    <article key={`${work.sessionId}-${i}`}>
                      <button className="voice-link" onClick={() => onOpenSession(work.sessionId)}>
                        {work.task.length > 120 ? `${work.task.slice(0, 120)}…` : work.task}
                      </button>
                      <details>
                        <summary>Task instructions</summary>
                        <p>{work.task}</p>
                      </details>
                    </article>
                  ))
                ) : (
                  <p className="voice-muted">
                    Independent work will appear here and in Agent Sessions.
                  </p>
                )}
                <h3>Latest exchanges</h3>
                <div className="voice-exchanges">
                  {[
                    ...conversation.inputs.map((u) => ({ ...u, who: 'You' })),
                    ...conversation.utterances.map((u) => ({ ...u, who: u.kind })),
                  ]
                    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
                    .slice(0, 20)
                    .map((u) => (
                      <article key={u.id}>
                        <small>{u.who}</small>
                        <p>{u.text}</p>
                      </article>
                    ))}
                </div>
                {conversation.playback && (
                  <p className="voice-muted">
                    Last playback: {conversation.playback.state} at{' '}
                    {(conversation.playback.playedMs / 1000).toFixed(1)} seconds.
                  </p>
                )}
              </>
            )}
          </section>
        )}
        {tab === 'setup' && (
          <VoiceSetupPanel
            view={view}
            settings={settings}
            busy={busy}
            onEdit={edit}
            onSave={() => void run(saveSettings)}
          />
        )}
      </div>
    </main>
  );
}
