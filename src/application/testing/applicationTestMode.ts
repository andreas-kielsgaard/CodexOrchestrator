export const APPLICATION_TEST_FEEDBACK_V1 = 'application_test_feedback/v1' as const;

export type ApplicationTestScalar = string | number | boolean | null;

export interface ApplicationTestScope {
  readonly mode: 'test_only';
  readonly buildRef: string;
  readonly testSessionId: string;
  readonly dataPolicy: 'synthetic_only';
  readonly allowedViewIds: readonly string[];
}

export interface ApplicationTestElement {
  readonly elementId: string;
  readonly label: string;
  readonly role: string;
  readonly actionIds: readonly string[];
}

export interface ApplicationTestObservation {
  readonly viewId: string;
  readonly observedAt: string;
  readonly state: Readonly<Record<string, ApplicationTestScalar>>;
  readonly elements: readonly ApplicationTestElement[];
}

export type ApplicationTestCondition =
  | {
      readonly kind: 'state_equals';
      readonly key: string;
      readonly value: ApplicationTestScalar;
    }
  | {
      readonly kind: 'state_at_least';
      readonly key: string;
      readonly value: number;
    }
  | {
      readonly kind: 'element_present';
      readonly elementId: string;
    };

export interface ApplicationTestAnnotationAnchor {
  readonly viewId: string;
  readonly elementId?: string;
  readonly stateKey?: string;
  readonly screenshotEvidenceId?: string;
  readonly timelineMs?: number;
}

export interface ApplicationTestAnnotation {
  readonly annotationId: string;
  readonly message: string;
  readonly anchor: ApplicationTestAnnotationAnchor;
  readonly createdAt: string;
}

export interface ApplicationTestFeedbackEnvelope {
  readonly version: typeof APPLICATION_TEST_FEEDBACK_V1;
  readonly authority: 'feedback_only';
  readonly source: 'application_test_mode';
  readonly buildRef: string;
  readonly testSessionId: string;
  readonly targetAgentSessionId: string;
  readonly annotations: readonly ApplicationTestAnnotation[];
  readonly screenshotEvidenceIds: readonly string[];
}

export interface ApplicationTestFeedbackDelivery {
  readonly status: 'delivered';
  readonly channel: 'agent_session_feedback' | 'recorded_test_sink';
  readonly envelope: ApplicationTestFeedbackEnvelope;
}

export interface ApplicationTestFeedbackSink {
  deliver(envelope: ApplicationTestFeedbackEnvelope): Promise<ApplicationTestFeedbackDelivery>;
}

export type ApplicationScreenshotCapture =
  | {
      readonly status: 'captured';
      readonly evidenceId: string;
      readonly mimeType: 'image/png';
      readonly buildRef: string;
      readonly testSessionId: string;
      readonly viewId: string;
      readonly elementId?: string;
    }
  | {
      readonly status: 'unavailable';
      readonly reason: string;
    };

export interface ApplicationDemoCaptureBoundary {
  readonly status: 'adapter_required';
  readonly buildRef: string;
  readonly testSessionId: string;
  readonly captureRootElementId: string;
  readonly excludedData: readonly string[];
  readonly reason: string;
}

export interface ApplicationTestModeController {
  readonly scope: ApplicationTestScope;
  navigate(viewId: string): Promise<ApplicationTestObservation>;
  observe(viewId: string): Promise<ApplicationTestObservation>;
  performAction(input: {
    readonly viewId: string;
    readonly elementId: string;
    readonly actionId: string;
  }): Promise<ApplicationTestObservation>;
  waitFor(input: {
    readonly viewId: string;
    readonly condition: ApplicationTestCondition;
    readonly timeoutMs: number;
  }): Promise<ApplicationTestObservation>;
  captureScreenshot(input: {
    readonly viewId: string;
    readonly elementId?: string;
  }): Promise<ApplicationScreenshotCapture>;
  describeDemoCapture(): Promise<ApplicationDemoCaptureBoundary>;
  addAnnotation(input: {
    readonly message: string;
    readonly anchor: ApplicationTestAnnotationAnchor;
  }): Promise<ApplicationTestAnnotation>;
  listAnnotations(): readonly ApplicationTestAnnotation[];
  deliverFeedback(targetAgentSessionId: string): Promise<ApplicationTestFeedbackDelivery>;
}
