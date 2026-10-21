import type { VoiceView } from '../../application/voiceSessions';

export function VoiceDiagnostics({
  view,
  onOpenSession,
}: {
  view: VoiceView;
  onOpenSession(id: string): void;
}) {
  const state = view.state;
  const captureFresh =
    !!view.diagnostics?.capture &&
    Date.now() - Date.parse(view.diagnostics.capture.updatedAt) < 5000;
  return (
    <details className="voice-connection-diagnostics">
      <summary>Connection and processing diagnostics</summary>
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
                  Last audio {new Date(view.diagnostics.audio.lastReceivedAt).toLocaleTimeString()}
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
    </details>
  );
}
