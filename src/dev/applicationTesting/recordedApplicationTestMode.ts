import {
  APPLICATION_TEST_FEEDBACK_V1,
  type ApplicationDemoCaptureBoundary,
  type ApplicationScreenshotCapture,
  type ApplicationTestAnnotation,
  type ApplicationTestCondition,
  type ApplicationTestFeedbackDelivery,
  type ApplicationTestFeedbackEnvelope,
  type ApplicationTestFeedbackSink,
  type ApplicationTestModeController,
  type ApplicationTestObservation,
  type ApplicationTestScope,
} from '../../application/testing';
import {
  createRecordedAgentSessionClient,
  type RecordedAgentSessionClient,
} from '../agentSessions/recordedAgentSessionClient';
import { recordedAgentSessionScenarios } from '../agentSessions/scenarios';

export const RECORDED_TEST_VIEW_ID = 'agent_sessions.workspace';

export interface RecordedApplicationTestModeComposition {
  readonly agentSessionClient: RecordedAgentSessionClient;
  readonly controller: ApplicationTestModeController;
  readonly deliveredFeedback: readonly ApplicationTestFeedbackEnvelope[];
}

export function createRecordedApplicationTestModeComposition(): RecordedApplicationTestModeComposition {
  const agentSessionClient = createRecordedAgentSessionClient({
    scenario: recordedAgentSessionScenarios.liveProcessing,
  });
  const deliveredFeedback: ApplicationTestFeedbackEnvelope[] = [];
  const feedbackSink: ApplicationTestFeedbackSink = {
    async deliver(envelope) {
      deliveredFeedback.push(structuredClone(envelope));
      return {
        status: 'delivered',
        channel: 'recorded_test_sink',
        envelope: structuredClone(envelope),
      };
    },
  };
  return {
    agentSessionClient,
    controller: createRecordedApplicationTestModeController(agentSessionClient, feedbackSink),
    deliveredFeedback,
  };
}

export function createRecordedApplicationTestModeController(
  agentSessionClient: RecordedAgentSessionClient,
  feedbackSink: ApplicationTestFeedbackSink,
  now: () => string = () => new Date().toISOString(),
): ApplicationTestModeController {
  const scope: ApplicationTestScope = {
    mode: 'test_only',
    buildRef: 'exploration-fixture/agent-testing-feedback-v1',
    testSessionId: 'test-session-agent-feedback',
    dataPolicy: 'synthetic_only',
    allowedViewIds: [RECORDED_TEST_VIEW_ID],
  };
  const annotations: ApplicationTestAnnotation[] = [];
  const screenshotEvidenceIds: string[] = [];
  let activeViewId: string | null = null;
  let nextAnnotation = 1;

  const observe = (viewId: string): ApplicationTestObservation => {
    assertView(viewId);
    const details = agentSessionClient.store.sessions.get('live-session');
    const invocation = details?.invocations.find(
      (entry) => entry.invocation.id === 'live-invocation',
    );
    return {
      viewId,
      observedAt: now(),
      state: {
        active: activeViewId === viewId,
        invocationStatus: invocation?.invocation.status ?? 'unavailable',
        eventCount: invocation?.events.length ?? 0,
        latestEventKind:
          invocation?.events.at(-1)?.normalized?.kind ?? invocation?.events.at(-1)?.source ?? null,
        remainingRecordedSteps: agentSessionClient.stepCount - agentSessionClient.stepIndex,
      },
      elements: [
        {
          elementId: 'agent_sessions.runtime_fixture',
          label: 'Recorded runtime step',
          role: 'control',
          actionIds: ['advance_recorded_runtime'],
        },
        {
          elementId: 'agent_sessions.status',
          label: 'Invocation status',
          role: 'status',
          actionIds: [],
        },
        {
          elementId: 'agent_sessions.transcript',
          label: 'Agent Session transcript',
          role: 'log',
          actionIds: [],
        },
      ],
    };
  };

  const controller: ApplicationTestModeController = {
    scope,
    async navigate(viewId) {
      assertView(viewId);
      activeViewId = viewId;
      return observe(viewId);
    },
    async observe(viewId) {
      return observe(viewId);
    },
    async performAction({ viewId, elementId, actionId }) {
      const current = observe(viewId);
      const element = current.elements.find((item) => item.elementId === elementId);
      if (!element?.actionIds.includes(actionId))
        throw new Error(`Semantic action is not available: ${elementId}/${actionId}`);
      if (actionId === 'advance_recorded_runtime' && !agentSessionClient.advance())
        throw new Error('The recorded runtime has no remaining step.');
      return observe(viewId);
    },
    async waitFor({ viewId, condition, timeoutMs }) {
      if (timeoutMs < 1 || timeoutMs > 5_000)
        throw new Error('Condition timeout must be between 1 and 5000 milliseconds.');
      const deadline = Date.now() + timeoutMs;
      do {
        const observation = observe(viewId);
        if (matches(observation, condition)) return observation;
        await new Promise((resolve) => setTimeout(resolve, 10));
      } while (Date.now() <= deadline);
      throw new Error(`Condition was not observed within ${timeoutMs}ms.`);
    },
    async captureScreenshot({ viewId, elementId }) {
      const current = observe(viewId);
      if (elementId && !current.elements.some((element) => element.elementId === elementId))
        throw new Error(`Unknown semantic element: ${elementId}`);
      return {
        status: 'unavailable',
        reason:
          'No app-window pixel capture adapter is connected. Broad desktop capture is disabled.',
      } satisfies ApplicationScreenshotCapture;
    },
    async describeDemoCapture() {
      return {
        status: 'adapter_required',
        buildRef: scope.buildRef,
        testSessionId: scope.testSessionId,
        captureRootElementId: 'agent_test_mode.preview_root',
        excludedData: [
          'other application windows',
          'product Agent Sessions',
          'user filesystem content',
        ],
        reason:
          'The proof defines the bounded capture envelope but does not record pixels or audio.',
      } satisfies ApplicationDemoCaptureBoundary;
    },
    async addAnnotation({ message, anchor }) {
      const observation = observe(anchor.viewId);
      if (!message.trim()) throw new Error('Annotation text is required.');
      if (
        anchor.elementId &&
        !observation.elements.some((element) => element.elementId === anchor.elementId)
      )
        throw new Error(`Unknown semantic element: ${anchor.elementId}`);
      const annotation = {
        annotationId: `annotation-${nextAnnotation++}`,
        message: message.trim(),
        anchor: structuredClone(anchor),
        createdAt: now(),
      };
      annotations.push(annotation);
      return structuredClone(annotation);
    },
    listAnnotations() {
      return structuredClone(annotations);
    },
    async deliverFeedback(targetAgentSessionId) {
      if (!targetAgentSessionId.trim()) throw new Error('A target Agent Session is required.');
      const envelope: ApplicationTestFeedbackEnvelope = {
        version: APPLICATION_TEST_FEEDBACK_V1,
        authority: 'feedback_only',
        source: 'application_test_mode',
        buildRef: scope.buildRef,
        testSessionId: scope.testSessionId,
        targetAgentSessionId: targetAgentSessionId.trim(),
        annotations: structuredClone(annotations),
        screenshotEvidenceIds: [...screenshotEvidenceIds],
      };
      return feedbackSink.deliver(envelope);
    },
  };

  return controller;

  function assertView(viewId: string): void {
    if (!scope.allowedViewIds.includes(viewId))
      throw new Error(`View is outside this test session: ${viewId}`);
  }
}

function matches(
  observation: ApplicationTestObservation,
  condition: ApplicationTestCondition,
): boolean {
  if (condition.kind === 'element_present')
    return observation.elements.some((element) => element.elementId === condition.elementId);
  const value = observation.state[condition.key];
  if (condition.kind === 'state_at_least')
    return typeof value === 'number' && value >= condition.value;
  return value === condition.value;
}

export type { ApplicationTestFeedbackDelivery };
