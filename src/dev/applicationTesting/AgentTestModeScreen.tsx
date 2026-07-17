import {
  AlertTriangle,
  Camera,
  Eye,
  FlaskConical,
  MessageSquare,
  Navigation,
  Play,
  Send,
  ShieldCheck,
  Timer,
  Video,
} from 'lucide-react';
import { useState } from 'react';
import type {
  ApplicationDemoCaptureBoundary,
  ApplicationScreenshotCapture,
  ApplicationTestAnnotation,
  ApplicationTestAnnotationAnchor,
  ApplicationTestFeedbackDelivery,
  ApplicationTestModeController,
  ApplicationTestObservation,
} from '../../application/testing';
import { StandaloneAgentSessionScreen } from '../../features/agentSessions/AgentSessionScreen';
import type { RecordedAgentSessionClient } from '../agentSessions/recordedAgentSessionClient';
import { RECORDED_TEST_VIEW_ID } from './recordedApplicationTestMode';
import './agentTestMode.css';

export interface AgentTestModeScreenProps {
  readonly agentSessionClient: RecordedAgentSessionClient;
  readonly controller: ApplicationTestModeController;
}

type AnchorChoice = 'view' | 'status' | 'transcript' | 'timeline';

export function AgentTestModeScreen({ agentSessionClient, controller }: AgentTestModeScreenProps) {
  const [observation, setObservation] = useState<ApplicationTestObservation | null>(null);
  const [capture, setCapture] = useState<ApplicationScreenshotCapture | null>(null);
  const [demoCapture, setDemoCapture] = useState<ApplicationDemoCaptureBoundary | null>(null);
  const [annotations, setAnnotations] = useState<readonly ApplicationTestAnnotation[]>([]);
  const [annotationText, setAnnotationText] = useState('');
  const [anchorChoice, setAnchorChoice] = useState<AnchorChoice>('status');
  const [targetSessionId, setTargetSessionId] = useState('agent-session-worker');
  const [delivery, setDelivery] = useState<ApplicationTestFeedbackDelivery | null>(null);
  const [activity, setActivity] = useState('Ready for an explicit semantic operation.');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run(
    label: string,
    operation: () => Promise<ApplicationTestObservation>,
  ): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      setObservation(await operation());
      setActivity(label);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function captureScreenshot(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      const result = await controller.captureScreenshot({
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'agent_sessions.transcript',
      });
      setCapture(result);
      setActivity(
        result.status === 'captured'
          ? `Captured ${result.evidenceId}.`
          : 'Screenshot request failed closed.',
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function inspectDemoBoundary(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      setDemoCapture(await controller.describeDemoCapture());
      setActivity('Inspected the bounded demo-capture envelope.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function addAnnotation(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      await controller.addAnnotation({
        message: annotationText,
        anchor: annotationAnchor(anchorChoice, observation),
      });
      setAnnotations(controller.listAnnotations());
      setAnnotationText('');
      setActivity('Added structured feedback without creating a product event.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function deliverFeedback(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      setDelivery(await controller.deliverFeedback(targetSessionId));
      setActivity('Delivered feedback to the recorded test sink.');
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="agent-test-mode">
      <main className="agent-test-mode__workspace">
        <header className="agent-test-mode__header">
          <div>
            <p className="eyebrow">Agent-native application evidence</p>
            <h1>Exercise the app through semantic controls</h1>
            <p className="agent-test-mode__description">
              This proof drives a recorded Agent Session component. It exposes no coordinates,
              production Agent Control commands, or broad desktop capture.
            </p>
            <p className="agent-test-mode__scope">
              <FlaskConical size={15} aria-hidden="true" />
              Development-only · synthetic data · feedback-only authority
            </p>
          </div>
          <dl>
            <div>
              <dt>Build</dt>
              <dd>{controller.scope.buildRef}</dd>
            </div>
            <div>
              <dt>Test session</dt>
              <dd>{controller.scope.testSessionId}</dd>
            </div>
          </dl>
        </header>

        <section className="agent-test-mode__grid">
          <aside className="agent-test-mode__controls" aria-label="Semantic test controls">
            <section>
              <header>
                <Navigation size={17} aria-hidden="true" />
                <h2>Semantic operations</h2>
              </header>
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run('Navigated to the Agent Sessions workspace.', () =>
                    controller.navigate(RECORDED_TEST_VIEW_ID),
                  )
                }
              >
                <Navigation size={15} aria-hidden="true" />
                Navigate to view
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run('Applied one recorded runtime step.', () =>
                    controller.performAction({
                      viewId: RECORDED_TEST_VIEW_ID,
                      elementId: 'agent_sessions.runtime_fixture',
                      actionId: 'advance_recorded_runtime',
                    }),
                  )
                }
              >
                <Play size={15} aria-hidden="true" />
                Advance semantic action
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run('Observed the current semantic state.', () =>
                    controller.observe(RECORDED_TEST_VIEW_ID),
                  )
                }
              >
                <Eye size={15} aria-hidden="true" />
                Observe state
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run('Observed at least one runtime event.', () =>
                    controller.waitFor({
                      viewId: RECORDED_TEST_VIEW_ID,
                      condition: { kind: 'state_at_least', key: 'eventCount', value: 1 },
                      timeoutMs: 500,
                    }),
                  )
                }
              >
                <Timer size={15} aria-hidden="true" />
                Wait for event
              </button>
            </section>

            <section>
              <header>
                <Camera size={17} aria-hidden="true" />
                <h2>Visual evidence</h2>
              </header>
              <button type="button" disabled={busy} onClick={() => void captureScreenshot()}>
                <Camera size={15} aria-hidden="true" />
                Capture screenshot
              </button>
              <button type="button" disabled={busy} onClick={() => void inspectDemoBoundary()}>
                <Video size={15} aria-hidden="true" />
                Inspect demo boundary
              </button>
              {capture && (
                <p className="agent-test-mode__boundary" role="status">
                  <AlertTriangle size={15} aria-hidden="true" />
                  {capture.status === 'unavailable'
                    ? capture.reason
                    : `Captured ${capture.evidenceId}.`}
                </p>
              )}
              {demoCapture && (
                <div className="agent-test-mode__boundary">
                  <ShieldCheck size={15} aria-hidden="true" />
                  <p>{demoCapture.reason}</p>
                  <small>Root: {demoCapture.captureRootElementId}</small>
                </div>
              )}
            </section>

            <section>
              <header>
                <MessageSquare size={17} aria-hidden="true" />
                <h2>Anchored feedback</h2>
              </header>
              <label>
                Anchor
                <select
                  value={anchorChoice}
                  onChange={(event) => setAnchorChoice(event.target.value as AnchorChoice)}
                >
                  <option value="view">Current view</option>
                  <option value="status">Status state</option>
                  <option value="transcript">Transcript element</option>
                  <option value="timeline">Recorded timeline point</option>
                </select>
              </label>
              <label>
                Annotation
                <textarea
                  value={annotationText}
                  onChange={(event) => setAnnotationText(event.target.value)}
                  placeholder="Describe the observed issue or requested adjustment."
                />
              </label>
              <button type="button" disabled={busy} onClick={() => void addAnnotation()}>
                <MessageSquare size={15} aria-hidden="true" />
                Add annotation
              </button>
              <label>
                Target Agent Session
                <input
                  value={targetSessionId}
                  onChange={(event) => setTargetSessionId(event.target.value)}
                />
              </label>
              <button type="button" disabled={busy} onClick={() => void deliverFeedback()}>
                <Send size={15} aria-hidden="true" />
                Deliver recorded feedback
              </button>
            </section>
          </aside>

          <section className="agent-test-mode__preview-column">
            <div className="agent-test-mode__activity" role={error ? 'alert' : 'status'}>
              <span className={error ? 'is-error' : undefined}>{error ?? activity}</span>
              {observation && (
                <code>
                  eventCount={String(observation.state.eventCount)} · status=
                  {String(observation.state.invocationStatus)}
                </code>
              )}
            </div>
            <div
              id="agent-test-preview-root"
              data-test-element-id="agent_test_mode.preview_root"
              className="agent-test-mode__preview"
              aria-label="Feature under test"
            >
              <StandaloneAgentSessionScreen client={agentSessionClient} />
            </div>
            <div className="agent-test-mode__feedback">
              <section>
                <h2>Annotations ({annotations.length})</h2>
                {annotations.length === 0 ? (
                  <p>No feedback has been anchored.</p>
                ) : (
                  <ol>
                    {annotations.map((annotation) => (
                      <li key={annotation.annotationId}>
                        <strong>{annotation.anchor.elementId ?? annotation.anchor.viewId}</strong>
                        <span>{annotation.message}</span>
                      </li>
                    ))}
                  </ol>
                )}
              </section>
              <section>
                <h2>Feedback envelope</h2>
                {delivery ? (
                  <pre>{JSON.stringify(delivery.envelope, null, 2)}</pre>
                ) : (
                  <p>Delivery is recorded in memory only. It is not an Orchestration Event.</p>
                )}
              </section>
            </div>
          </section>
        </section>
      </main>
    </div>
  );
}

function annotationAnchor(
  choice: AnchorChoice,
  observation: ApplicationTestObservation | null,
): ApplicationTestAnnotationAnchor {
  switch (choice) {
    case 'status':
      return {
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'agent_sessions.status',
        stateKey: 'invocationStatus',
      };
    case 'transcript':
      return {
        viewId: RECORDED_TEST_VIEW_ID,
        elementId: 'agent_sessions.transcript',
      };
    case 'timeline':
      return {
        viewId: RECORDED_TEST_VIEW_ID,
        timelineMs: Number(observation?.state.eventCount ?? 0) * 1_000,
      };
    default:
      return { viewId: RECORDED_TEST_VIEW_ID };
  }
}
