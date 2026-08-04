import { render, screen } from '@testing-library/react';
import type {
  ProductWorkUnitHandlerDecisionV1,
  ProductWorkUnitHandlerReviewV1,
  ProductWorkUnitImplementerOutcomeV1,
  SprintWorkspacePresentationV1,
} from '../../../application/orchestrations';
import { WorkUnitDetailWorkspace } from './WorkUnitDetailWorkspace';

type PresentedWorkUnit =
  SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];

describe('WorkUnitDetailWorkspace Implementer outcome activity', () => {
  it('keeps absence as absence and shows an in-progress reporting continuation without later facts', () => {
    const rendered = renderWorkspace();
    expect(screen.queryByText('Implementer reporting')).toBeNull();
    expect(screen.queryByText('Ready for Handler review')).toBeNull();

    rendered.rerender(workspace(reportingOutcome()));
    const detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Implementer reporting invocation is prepared.');
    expect(detail).toHaveTextContent('Requested');
    expect(detail).toHaveTextContent('Prepared');
    expect(detail).not.toHaveTextContent('Submitted outcome claims');
    expect(detail).not.toHaveTextContent('Application-owned File Review evidence');
    expect(screen.queryByText('Ready for Handler review')).toBeNull();
  });

  it.each(['failed', 'canceled'] as const)(
    'shows a %s reporting terminal without application acceptance or review readiness',
    (status) => {
      renderWorkspace(terminalOutcome(status));
      const detail = screen.getByLabelText('Work Unit context');
      expect(detail).toHaveTextContent(
        `Reporting lifecycle was observed as ${status === 'failed' ? 'Failed' : 'Canceled'}`,
      );
      expect(detail).not.toHaveTextContent('application-accepted');
      expect(screen.queryByText('Ready for Handler review')).toBeNull();
    },
  );

  it('labels Implementer prose as claims and shows authoritative evidence before exact review readiness', () => {
    renderWorkspace(reviewReadyOutcome());
    const detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Submitted outcome claims');
    expect(detail).toHaveTextContent('Implementer summary claim');
    expect(detail).toHaveTextContent('Implemented the bounded change.');
    expect(detail).toHaveTextContent('Implementer validation claim');
    expect(detail).toHaveTextContent('Focused checks passed.');
    expect(detail).toHaveTextContent(
      'These Implementer statements are claims, not application-owned evidence.',
    );
    expect(detail).toHaveTextContent('Application-owned File Review evidence');
    expect(detail).toHaveTextContent('Reporting invocation');
    expect(detail).toHaveTextContent('reporting-invocation-1');
    expect(detail).toHaveTextContent('Reporting Harness revision');
    expect(detail).toHaveTextContent('reporting-revision-1');
    expect(detail).toHaveTextContent('src/feature.ts');
    expect(detail).toHaveTextContent('evidence reference evidence-1');
    expect(detail).toHaveTextContent('content fingerprint content-1');
    expect(detail).toHaveTextContent('Comparison fingerprint: comparison-1');
    expect(detail).toHaveTextContent('Semantic outcome completion was recorded');
    expect(detail).toHaveTextContent('Reporting lifecycle was observed as Completed');
    expect(detail).toHaveTextContent('The reporting outcome was application-accepted');
    expect(screen.getByText('Ready for Handler review')).toBeVisible();
    expect(detail).toHaveTextContent('No Handler judgment is recorded here.');
    expect(detail).not.toHaveTextContent(
      /implementation approved|Work Unit accepted|returned for correction|retry requested|settled|dependency activated|Sprint continuation/i,
    );
  });

  it('shows pending, accepted, and returned Handler review facts without later workflow', () => {
    const rendered = renderWorkspace(reviewReadyOutcome(), handlerReview('pending'));
    let detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Handler review is application-ready.');
    expect(detail).toHaveTextContent('Handler semantic judgment is pending');
    expect(detail).toHaveTextContent('Application-bound claims and evidence');
    expect(detail).toHaveTextContent('src/feature.ts');
    expect(detail).toHaveTextContent('No retry attempt, settlement, dependent activation, or upward continuation is recorded.');

    rendered.rerender(workspace(reviewReadyOutcome(), handlerReview('failed')));
    detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Handler review lifecycle was observed as Failed');
    expect(detail).not.toHaveTextContent('Handler decision:');

    const accepted = handlerReview('accepted');
    rendered.rerender(workspace(reviewReadyOutcome(), accepted, handlerDecision('accepted')));
    detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Handler semantic judgment was recorded as accept');
    expect(detail).toHaveTextContent('Handler decision: accepted');
    expect(detail).not.toHaveTextContent('Structured return reason:');

    rendered.rerender(
      workspace(
        reviewReadyOutcome(),
        {
          ...handlerReview('returned'),
          conflict: { occurredAt: '2026-08-04T00:00:20Z', reason: 'divergent_review_judgment' },
        },
        handlerDecision('returned'),
      ),
    );
    detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Handler semantic judgment was recorded as return');
    expect(detail).toHaveTextContent('Handler decision: returned');
    expect(detail).toHaveTextContent('Structured return reason: review_failed');
    expect(detail).toHaveTextContent('Retry is required');
    expect(detail).toHaveTextContent('Review conflict observed at');
    expect(detail).not.toHaveTextContent(/implementation approved|Work Unit accepted|settled|Sprint continuation/i);
  });
});

function renderWorkspace(
  outcome?: ProductWorkUnitImplementerOutcomeV1,
  handlerReview?: ProductWorkUnitHandlerReviewV1,
  handlerDecision?: ProductWorkUnitHandlerDecisionV1,
) {
  return render(workspace(outcome, handlerReview, handlerDecision));
}

function workspace(
  outcome?: ProductWorkUnitImplementerOutcomeV1,
  handlerReview?: ProductWorkUnitHandlerReviewV1,
  handlerDecision?: ProductWorkUnitHandlerDecisionV1,
) {
    return (
    <WorkUnitDetailWorkspace
      unit={presentedWorkUnit(outcome, handlerReview, handlerDecision)}
      lifecycleEntries={[]}
      workSlicePlanningPointGroupTitle="Planning point"
      sessions={[]}
      onBack={vi.fn()}
    />
  );
}

function presentedWorkUnit(
  outcome?: ProductWorkUnitImplementerOutcomeV1,
  handlerReview?: ProductWorkUnitHandlerReviewV1,
  handlerDecision?: ProductWorkUnitHandlerDecisionV1,
): PresentedWorkUnit {
  return {
    workUnitId: 'unit-1',
    title: 'Bounded responsibility',
    summary: 'Implement one bounded change.',
    details: 'Accepted Work Slice responsibility.',
    source: {
      status: 'available',
      sourceKind: 'repository',
      sourceReferences: ['materialization-1'],
    },
    ...(outcome ? { implementerOutcome: outcome } : {}),
    ...(handlerReview ? { handlerReview } : {}),
    ...(handlerDecision ? { handlerDecision } : {}),
    workUnitScopeId: 'scope-1',
    sprintPlanRevisionId: 'revision-1',
    fixedExecutionScopeIds: [],
    dependencies: [],
    gateIds: [],
    attempts: [],
    reviews: [],
    observed: {
      executionRequested: false,
      launched: false,
      returned: false,
      integrated: false,
      responsibilityAccepted: false,
    },
    presentationState: 'not_started',
  };
}

function reportingOutcome(): ProductWorkUnitImplementerOutcomeV1 {
  return {
    attemptId: 'attempt-1',
    implementerSessionId: 'implementer-session-1',
    originalImplementerInvocationId: 'implementer-invocation-1',
    reportingInvocationId: 'reporting-invocation-1',
    reportingHarnessRevisionId: 'reporting-revision-1',
    reportingHarnessConfigurationDigest: 'reporting-digest-1',
    reportingHarnessRepositoryCommitRef: 'reporting-commit-1',
    reportingRequestedAt: '2026-08-04T00:00:00Z',
    reportingPreparedAt: '2026-08-04T00:00:01Z',
  };
}

function terminalOutcome(status: 'failed' | 'canceled'): ProductWorkUnitImplementerOutcomeV1 {
  return {
    ...reportingOutcome(),
    reportingHarnessBoundAt: '2026-08-04T00:00:02Z',
    reportingLaunchRequestedAt: '2026-08-04T00:00:03Z',
    reportingLaunchAcceptedAt: '2026-08-04T00:00:04Z',
    reportingReadyAt: '2026-08-04T00:00:05Z',
    terminalLifecycle: { status, observedAt: '2026-08-04T00:00:09Z' },
  };
}

function reviewReadyOutcome(): ProductWorkUnitImplementerOutcomeV1 {
  return {
    ...terminalOutcome('failed'),
    submittedOutcome: {
      variant: 'review_pending',
      summaryClaim: 'Implemented the bounded change.',
      validationStatementClaim: 'Focused checks passed.',
      semanticPayloadFingerprint: 'payload-1',
      submittedAt: '2026-08-04T00:00:06Z',
      validationAt: '2026-08-04T00:00:06Z',
      validationResult: 'valid',
    },
    evidence: {
      changedFiles: [
        {
          evidenceRef: 'evidence-1',
          displayName: 'src/feature.ts',
          changeKind: 'modified',
          contentFingerprint: 'content-1',
        },
      ],
      comparisonFingerprint: 'comparison-1',
      readyAt: '2026-08-04T00:00:07Z',
    },
    semanticCompletion: {
      invocationId: 'reporting-invocation-1',
      completedAt: '2026-08-04T00:00:08Z',
    },
    terminalLifecycle: { status: 'completed', observedAt: '2026-08-04T00:00:09Z' },
    applicationAcceptedAt: '2026-08-04T00:00:10Z',
    handlerReviewReadyAt: '2026-08-04T00:00:11Z',
  };
}

function handlerReview(
  state: 'pending' | 'accepted' | 'returned' | 'failed',
): ProductWorkUnitHandlerReviewV1 {
  return {
    attemptId: 'attempt-1',
    reportingInvocationId: 'reporting-invocation-1',
    handlerSessionId: 'handler-session-1',
    originalHandlerInvocationId: 'handler-invocation-1',
    actionHandlerInvocationId: 'handler-action-1',
    reviewInvocationId: 'review-invocation-1',
    reviewHarnessRevisionId: 'review-revision-1',
    reviewHarnessConfigurationDigest: 'review-digest-1',
    reviewHarnessRepositoryCommitRef: 'review-commit-1',
    deliveryRequestedAt: '2026-08-04T00:00:12Z',
    deliveryPersistedAt: '2026-08-04T00:00:12Z',
    harnessBoundAt: '2026-08-04T00:00:13Z',
    launchRequestedAt: '2026-08-04T00:00:14Z',
    launchAcceptedAt: '2026-08-04T00:00:15Z',
    reviewReadyAt: '2026-08-04T00:00:16Z',
    delivered: {
      summaryClaim: 'Implemented the bounded change.',
      validationStatementClaim: 'Focused checks passed.',
      changedFiles: [
        {
          evidenceRef: 'evidence-1',
          displayName: 'src/feature.ts',
          changeKind: 'modified',
          contentFingerprint: 'content-1',
        },
      ],
      comparisonFingerprint: 'comparison-1',
      deliveredPayloadFingerprint: 'delivery-1',
    },
    ...(state === 'pending'
      ? {}
      : state === 'failed'
        ? { lifecycle: { status: 'failed' as const, observedAt: '2026-08-04T00:00:18Z' } }
      : {
          semanticJudgment: {
            variant: state === 'accepted' ? ('accept' as const) : ('return' as const),
            ...(state === 'returned'
              ? { reason: { code: 'review_failed', explanation: 'Evidence requires correction.' } }
              : {}),
            fingerprint: 'judgment-1',
            recordedAt: '2026-08-04T00:00:17Z',
          },
          lifecycle: { status: 'completed' as const, observedAt: '2026-08-04T00:00:18Z' },
        }),
  };
}

function handlerDecision(
  variant: 'accepted' | 'returned',
): ProductWorkUnitHandlerDecisionV1 {
  return variant === 'accepted'
    ? {
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        recordedAt: '2026-08-04T00:00:19Z',
        implementationAcceptedAt: '2026-08-04T00:00:19Z',
      }
    : {
        reviewInvocationId: 'review-invocation-1',
        variant,
        fingerprint: 'decision-1',
        returnReason: { code: 'review_failed', explanation: 'Evidence requires correction.' },
        recordedAt: '2026-08-04T00:00:19Z',
        implementationReturnedAt: '2026-08-04T00:00:19Z',
        retryRequiredAt: '2026-08-04T00:00:19Z',
      };
}
