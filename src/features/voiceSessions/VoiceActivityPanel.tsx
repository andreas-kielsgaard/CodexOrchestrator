import { useState } from 'react';
import type { VoiceView } from '../../application/voiceSessions';
import { VoiceDiagnostics } from './VoiceDiagnostics';

export function VoiceActivityPanel({
  view,
  busy,
  onSend,
  onOpenSession,
  onShowRoutes,
}: {
  view: VoiceView;
  busy: boolean;
  onSend(text: string): Promise<boolean>;
  onOpenSession(id: string, invocationId?: string): void;
  onShowRoutes(): void;
}) {
  const [text, setText] = useState('');
  const conversation = view.state.conversation;
  const exchanges = [
    ...(conversation?.inputs.map((input) => ({ ...input, who: 'You' })) ?? []),
    ...(conversation?.utterances.map((utterance) => ({
      ...utterance,
      who: `Orchid · published ${utterance.kind}`,
    })) ?? []),
  ]
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    .slice(0, 20);
  return (
    <section className="voice-panel voice-activity" aria-label="Conversation activity">
      <div className="voice-workflow-toolbar">
        <div>
          <h2>Conversation activity</h2>
          <p>Requests, responses and independent work.</p>
        </div>
        <button onClick={onShowRoutes}>Inspect recorded routes</button>
      </div>
      {view.diagnostics && (
        <p>
          <strong>{view.diagnostics.input.title}</strong> · {view.diagnostics.input.detail}
        </p>
      )}
      {view.diagnostics?.filterError && (
        <p role="alert" className="voice-error">
          JEV: {view.diagnostics.filterError} · input retained for reevaluation
        </p>
      )}
      {view.diagnostics?.stages
        .filter((stage) => stage.error)
        .map((stage) => (
          <p className="voice-error" key={stage.sessionId}>
            {stage.role}: {stage.error}{' '}
            <button onClick={() => onOpenSession(stage.sessionId)}>Inspect failed stage</button>
          </p>
        ))}
      <form
        className="voice-test-request"
        onSubmit={async (e) => {
          e.preventDefault();
          if (await onSend(text)) setText('');
        }}
      >
        <label>
          Try a text request
          <input
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Describe what you want naturally"
          />
        </label>
        <button disabled={busy || !view.state.enabled || !text.trim()}>Evaluate request</button>
      </form>
      <p className="voice-muted">
        Uses the real voice pipeline and may delegate work.{' '}
        {view.state.enabled ? '' : 'Start the connection to try a request.'}
      </p>
      <div className="voice-activity-columns">
        <section>
          <h3>Latest exchanges</h3>
          {!exchanges.length && (
            <p>No exchanges yet. Start a connection and speak, or try a text request.</p>
          )}
          {exchanges.map((exchange) => (
            <article key={exchange.id}>
              <small>
                {exchange.who} · {new Date(exchange.createdAt).toLocaleTimeString()}
              </small>
              <p>{exchange.text}</p>
            </article>
          ))}
          {conversation?.playback && (
            <p className="voice-muted">
              Last playback: {conversation.playback.state} at{' '}
              {(conversation.playback.playedMs / 1000).toFixed(1)} seconds.
            </p>
          )}
        </section>
        <section aria-label="Voice tasks">
          <h3>Work and outcomes</h3>
          {!conversation?.tasks?.length && <p>No tracked work yet.</p>}
          {conversation?.tasks?.map((task) => {
            const assignments = conversation.delegations.filter((a) => a.taskId === task.id);
            const latest = assignments.at(-1);
            const reported = [...assignments].reverse().find((a) => a.outcome);
            return (
              <article key={task.id}>
                <strong>{task.title}</strong>
                <p>
                  Request {task.revision} · {latest?.runtimeStatus ?? 'pending'}
                  {latest?.delivery?.inputId && ` · amendment ${latest.delivery.state}`}
                </p>
                {reported?.outcome && (
                  <div>
                    <strong>
                      {reported.outcome.revision < task.revision
                        ? 'Earlier outcome'
                        : 'Reported outcome'}{' '}
                      · request {reported.outcome.revision}
                    </strong>
                    <p>{reported.outcome.summary}</p>
                  </div>
                )}
                <details>
                  <summary>Current assignment instructions</summary>
                  <p>{task.request}</p>
                </details>
                {latest && (
                  <button
                    onClick={() =>
                      onOpenSession(latest.sessionId, latest.invocationId ?? undefined)
                    }
                  >
                    Open worker
                  </button>
                )}
              </article>
            );
          })}
          {!!conversation?.pendingTopics.length && (
            <>
              <h3>Ready to discuss</h3>
              {conversation.pendingTopics.map((topic) => (
                <article key={topic.id}>
                  <strong>{topic.title}</strong>
                  <details>
                    <summary>Source material</summary>
                    <p>{topic.material}</p>
                  </details>
                </article>
              ))}
            </>
          )}
          {!!conversation?.inputBuffer?.blocks.length && (
            <section>
              <h3>Input held for completion</h3>
              {conversation.inputBuffer.blocks.map((block) => (
                <p key={block.id}>{block.text}</p>
              ))}
              <p className="voice-muted">
                {conversation.inputBuffer.complete
                  ? 'Ready · waiting for speech and queued processing'
                  : 'Waiting for the input to be complete'}
              </p>
            </section>
          )}
          {!!conversation?.delegations.length && (
            <details>
              <summary>All assignments · {conversation.delegations.length}</summary>
              {conversation.delegations.map((work, i) => (
                <article key={work.id ?? `${work.sessionId}-${i}`}>
                  <p>{work.task}</p>
                  <button
                    onClick={() => onOpenSession(work.sessionId, work.invocationId ?? undefined)}
                  >
                    Open assignment
                  </button>
                </article>
              ))}
            </details>
          )}
        </section>
      </div>
      <VoiceDiagnostics view={view} onOpenSession={onOpenSession} />
    </section>
  );
}
