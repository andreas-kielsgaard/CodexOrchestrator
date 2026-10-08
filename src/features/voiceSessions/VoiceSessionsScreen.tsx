import { useEffect, useState } from 'react';
import type { VoicePrompts, VoiceSessionClient, VoiceSettings, VoiceView } from '../../application/voiceSessions';
import './voiceSessions.css';

export function VoiceSessionsScreen({
  client,
  onOpenSession,
}: {
  client: VoiceSessionClient;
  onOpenSession: (id: string) => void;
}) {
  const [view, setView] = useState<VoiceView>();
  const [settings, setSettings] = useState<VoiceSettings>();
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [text, setText] = useState('');
  const [promptStage, setPromptStage] = useState<keyof VoicePrompts>('relevance');
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
        <h1>Voice Sessions</h1>
        <p>{error || 'Loading conversation…'}</p>
      </main>
    );
  const state = view.state;
  const conversation = state.conversation;
  const captureFresh = !!view.diagnostics?.capture && Date.now() - Date.parse(view.diagnostics.capture.updatedAt) < 5000;
  const edit = (change: Partial<VoiceSettings>) => setSettings({ ...settings, ...change });
  return (
    <main className="voice-screen">
      <header>
        <div>
          <h1>Voice Sessions</h1>
          <p>Reason together while independent agents keep working.</p>
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
      <div className="voice-columns">
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
              <p><strong>{view.diagnostics.input.title}</strong><br />{view.diagnostics.input.detail}</p>
              <dl>
                <dt>Microphone</dt>
                <dd>{!state.phoneConnected ? 'Phone disconnected' : !view.diagnostics.capture ? 'Connected · waiting for capture status from the phone' :
                  <>{view.diagnostics.capture.routedMicrophone} · {!captureFresh ? 'capture update missing' : view.diagnostics.capture.recording ? 'recording' : 'stopped'}
                    {view.diagnostics.capture.silenced && ' · blocked by Android'}
                    {view.diagnostics.capture.requestedMicrophone === 'headset' && !view.diagnostics.capture.external && ' · external requested, phone fallback'}
                    <br />Input signal {(view.diagnostics.capture.level * 100).toFixed(1)}% · {view.diagnostics.capture.segmentsSent} packets sent
                    {view.diagnostics.capture.error && <p className="voice-error">{view.diagnostics.capture.error}</p>}
                  </>}</dd>
                <dt>Transcription</dt>
                <dd>{view.diagnostics.audio.received} audio packets received · {view.diagnostics.audio.transcribed} context windows decoded · {view.diagnostics.pendingAudio} queued
                  {view.diagnostics.audio.lastReceivedAt && <><br />Last audio {new Date(view.diagnostics.audio.lastReceivedAt).toLocaleTimeString()}</>}
                  {view.diagnostics.audio.lastTranscript != null && <><br />Last transcript: {view.diagnostics.audio.lastTranscript || '(no speech recognized)'}</>}
                  {view.diagnostics.audio.error && <p className="voice-error">{view.diagnostics.audio.error}</p>}
                </dd>
                <dt>JEV decision</dt>
                <dd>{view.diagnostics.input.last ? <>{view.diagnostics.input.last.filterPending ? 'Evaluating' : view.diagnostics.input.last.relevance} · {view.diagnostics.input.last.completion ?? 'completion not checked'}<br />{view.diagnostics.input.last.text}</> : 'No input evaluated yet'}</dd>
              </dl>
              <div className="voice-stage-status">
                {view.diagnostics.stages.map((stage) => <div key={stage.sessionId}>
                  <button className="voice-link" onClick={() => onOpenSession(stage.sessionId)}>{stage.role} · {stage.status ?? 'not started'}{stage.queued > 0 ? ` · ${stage.queued} queued` : ''}</button>
                  {stage.error && <p className="voice-error">{stage.error}</p>}
                </div>)}
              </div>
              <small className="voice-muted">Updated {new Date(view.diagnostics.updatedAt).toLocaleTimeString()}</small>
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
            <button disabled={busy || !state.enabled} onClick={() => void run(() => client.stop())}>
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
                      {conversation.inputBuffer.blocks.map((input) => <p key={input.id}>{input.text}</p>)}
                      <p className="voice-muted">{conversation.inputBuffer.complete ? 'Ready · waiting for speech and queued processing' : 'Waiting for the input to be complete'}</p>
                    </details>
                  )}
                  <details>
                    <summary>Input decisions</summary>
                    {conversation.inputs.slice(-10).map((input) => (
                      <p key={input.id}>{input.text} · {input.filterPending ? 'Evaluating' : input.relevance ?? 'Earlier input'} {input.completion ? '· ' + input.completion : ''}</p>
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
              <h3>Delegated work</h3>
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
        <section className="voice-panel">
          <h2>Phone setup</h2>
          <p>Connect both devices to Tailscale. Keep Orchid running and the laptop awake.</p>
          <label>
            Laptop Tailscale address and port
            <input
              disabled={state.enabled}
              value={settings.bindAddress}
              onChange={(event) => edit({ bindAddress: event.target.value })}
              placeholder="100.x.x.x:4818"
            />
          </label>
          <label>
            Phone endpoint
            <input readOnly value={`ws://${settings.bindAddress}/phone`} />
          </label>
          <label>
            Access token
            <input type="password" readOnly value={settings.token} />
          </label>
          <button onClick={() => void navigator.clipboard.writeText(settings.token)}>
            Copy token
          </button>
          <label>
            Capability Profile
            <select
              disabled={state.enabled}
              value={settings.profileId}
              onChange={(event) => edit({ profileId: event.target.value })}
            >
              <option value="">Choose a laptop profile</option>
              {view.profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Working directory
            <input
              disabled={state.enabled}
              value={settings.workingDirectory ?? ''}
              onChange={(event) => edit({ workingDirectory: event.target.value || null })}
              placeholder="Leave empty for an Orchid workspace"
            />
          </label>
          <p className="voice-muted">
            Conversation agents receive Orchid session tools for discovery, delegation, steering,
            and runtime requests.
          </p>
          <details>
            <summary>Voice prompts</summary>
            <label>
              Stage
              <select value={promptStage} onChange={(event) => setPromptStage(event.target.value as keyof VoicePrompts)}>
                <option value="relevance">JEV relevance</option>
                <option value="completion">JEV input completion</option>
                <option value="continuation">JEV continuation</option>
                <option value="summary">Input summarization</option>
                <option value="router">Routing</option>
                <option value="digest">Digest</option>
                <option value="output">Output</option>
                <option value="tools">Shared tool discovery</option>
              </select>
            </label>
            <label>
              Prompt
              <textarea
                disabled={state.enabled}
                rows={16}
                value={settings.prompts[promptStage]}
                onChange={(event) => edit({ prompts: { ...settings.prompts, [promptStage]: event.target.value } })}
              />
            </label>
            <button disabled={busy || state.enabled} onClick={() => edit({ prompts: { ...settings.prompts, [promptStage]: view.defaultPrompts[promptStage] } })}>
              Reset this prompt
            </button>
            <p className="voice-muted">Stop the connection to edit. Save setup applies prompts to subsequent evaluations and agent turns.</p>
          </details>
          <details>
            <summary>Routing preferences</summary>
            <textarea
              disabled={state.enabled}
              rows={16}
              value={settings.instructions}
              onChange={(event) => edit({ instructions: event.target.value })}
            />
          </details>
          <button
            disabled={busy || state.enabled}
            onClick={() => void run(() => client.configure(settings))}
          >
            Save setup
          </button>
        </section>
      </div>
    </main>
  );
}
