import { useEffect, useState } from 'react';
import type {
  ProductWorkUnitHandlerDecisionV1,
  ProductWorkUnitHandlerReviewV1,
  ProductWorkUnitImplementerOutcomeV1,
  ProductWorkUnitIntegrationV1,
  SprintWorkspacePresentationV1,
} from '../../application/orchestrations';
import { WorkUnitDetailWorkspace } from '../../features/orchestrations/components/WorkUnitDetailWorkspace';
import './operationalSpineCheckpointDemo.css';

type PresentedWorkUnit =
  SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];

const stages = [
  {
    id: 'materialized',
    label: '1. Materialized',
    description: 'An accepted Work Slice responsibility now exists as one durable Work Unit.',
  },
  {
    id: 'implementation-ready',
    label: '2. Implementation ready',
    description: 'The application has prepared the Handler and a narrowly scoped Implementer.',
  },
  {
    id: 'review-ready',
    label: '3. Review ready',
    description: 'Implementer claims and application-owned file evidence are ready for review.',
  },
  {
    id: 'handler-accepted',
    label: '4. Handler accepted',
    description: 'An independent Handler decision accepts the exact reviewed candidate.',
  },
  {
    id: 'integrated',
    label: '5. Integrated and settled',
    description: 'The accepted candidate is integrated and contributes to dependent readiness.',
  },
] as const;

type StageId = (typeof stages)[number]['id'];

export function OperationalSpineCheckpointDemo() {
  const [stage, setStage] = useState<StageId>(requestedStage());
  const selected = stages.find((candidate) => candidate.id === stage)!;

  useEffect(() => {
    if (new URLSearchParams(window.location.search).get('focus') !== 'integration') return;
    requestAnimationFrame(() => {
      const context = document.querySelector<HTMLElement>('[aria-label="Work Unit context"]');
      context?.scrollTo({ top: context.scrollHeight });
    });
  }, [stage]);

  return (
    <div className="checkpoint-demo">
      <header className="checkpoint-demo__header">
        <div>
          <p className="checkpoint-demo__eyebrow">Controlled fixture · accepted checkpoint 55cdd40</p>
          <h1>One Work Unit, from accepted plan to settled integration</h1>
          <p>{selected.description}</p>
        </div>
        <p className="checkpoint-demo__boundary">
          This presents implemented states with deterministic fixture data. It does not claim a live
          provider run, dependent activation, Work Slice settlement, publication, or user acceptance.
        </p>
        <nav aria-label="Demonstration stages">
          {stages.map((candidate) => (
            <button
              key={candidate.id}
              type="button"
              aria-pressed={candidate.id === stage}
              onClick={() => setStage(candidate.id)}
            >
              {candidate.label}
            </button>
          ))}
        </nav>
      </header>
      <div className="checkpoint-demo__surface">
        <WorkUnitDetailWorkspace
          unit={unitForStage(stage)}
          lifecycleEntries={[]}
          workSlicePlanningPointGroupTitle="Accepted planning point"
          sessions={[]}
          backLabel="Return to first stage"
          onBack={() => setStage('materialized')}
        />
      </div>
    </div>
  );
}

function requestedStage(): StageId {
  const requested = new URLSearchParams(window.location.search).get('stage');
  return stages.some((stage) => stage.id === requested) ? (requested as StageId) : 'materialized';
}

function unitForStage(stage: StageId): PresentedWorkUnit {
  const base: PresentedWorkUnit = {
    workUnitId: 'wu-operational-spine-demo',
    title: 'Add one bounded orchestration capability',
    summary: 'Implement, review, and integrate one accepted responsibility.',
    details: 'Controlled fixture derived from the accepted Work Unit execution checkpoint.',
    source: {
      status: 'available',
      sourceKind: 'repository',
      sourceReferences: ['accepted-work-slice-revision'],
    },
    workUnitScopeId: 'scope-demo-1',
    sprintPlanRevisionId: 'sprint-revision-demo-1',
    fixedExecutionScopeIds: [],
    dependencies: [],
    gateIds: [],
    attempts: [],
    reviews: [],
    observed: {
      executionRequested: stage !== 'materialized',
      launched: stage !== 'materialized',
      returned: false,
      integrated: stage === 'integrated',
      responsibilityAccepted: stage === 'integrated',
    },
    presentationState:
      stage === 'materialized'
        ? 'not_started'
        : stage === 'review-ready' || stage === 'handler-accepted'
          ? 'under_review'
          : stage === 'integrated'
            ? 'integrated'
            : 'launched',
  };

  if (stage === 'materialized') return base;

  const ready: PresentedWorkUnit = {
    ...base,
    handlerActivation: {
      eligibilityState: 'eligible',
      stage: 'handler_ready',
      providerActivityObserved: false,
    },
    actionContinuation: {
      stage: 'action_ready',
      providerActivityObserved: false,
    },
    implementerActivation: {
      stage: 'implementer_ready',
      providerActivityObserved: false,
    },
  };

  if (stage === 'implementation-ready') return ready;

  const withOutcome: PresentedWorkUnit = {
    ...ready,
    implementerOutcome: reviewReadyOutcome(),
    handlerReview: handlerReview(stage === 'review-ready' ? 'pending' : 'accepted'),
  };

  if (stage === 'review-ready') return withOutcome;

  const accepted: PresentedWorkUnit = {
    ...withOutcome,
    handlerDecision: handlerDecision(),
  };

  if (stage === 'handler-accepted') return accepted;

  return {
    ...accepted,
    integration: integrationResult(),
  };
}

function reviewReadyOutcome(): ProductWorkUnitImplementerOutcomeV1 {
  return {
    attemptId: 'attempt-demo-1',
    implementerSessionId: 'implementer-session-demo-1',
    originalImplementerInvocationId: 'implementer-invocation-demo-1',
    reportingInvocationId: 'reporting-invocation-demo-1',
    reportingHarnessRevisionId: 'reporting-revision-demo-1',
    reportingHarnessConfigurationDigest: 'reporting-digest-demo-1',
    reportingHarnessRepositoryCommitRef: 'accepted-checkpoint-55cdd40',
    reportingRequestedAt: '2026-08-04T08:00:00Z',
    reportingPreparedAt: '2026-08-04T08:00:01Z',
    reportingHarnessBoundAt: '2026-08-04T08:00:02Z',
    reportingLaunchRequestedAt: '2026-08-04T08:00:03Z',
    reportingLaunchAcceptedAt: '2026-08-04T08:00:04Z',
    reportingReadyAt: '2026-08-04T08:00:05Z',
    submittedOutcome: {
      variant: 'review_pending',
      summaryClaim: 'Implemented the bounded orchestration capability.',
      validationStatementClaim: 'Focused deterministic checks passed.',
      semanticPayloadFingerprint: 'payload-demo-1',
      submittedAt: '2026-08-04T08:00:06Z',
      validationAt: '2026-08-04T08:00:06Z',
      validationResult: 'valid',
    },
    evidence: {
      changedFiles: [
        {
          evidenceRef: 'evidence-demo-1',
          displayName: 'src/orchestration-capability.ts',
          changeKind: 'modified',
          contentFingerprint: 'content-demo-1',
        },
      ],
      comparisonFingerprint: 'comparison-demo-1',
      readyAt: '2026-08-04T08:00:07Z',
    },
    semanticCompletion: {
      invocationId: 'reporting-invocation-demo-1',
      completedAt: '2026-08-04T08:00:08Z',
    },
    terminalLifecycle: { status: 'completed', observedAt: '2026-08-04T08:00:09Z' },
    applicationAcceptedAt: '2026-08-04T08:00:10Z',
    handlerReviewReadyAt: '2026-08-04T08:00:11Z',
  };
}

function handlerReview(state: 'pending' | 'accepted'): ProductWorkUnitHandlerReviewV1 {
  return {
    attemptId: 'attempt-demo-1',
    reportingInvocationId: 'reporting-invocation-demo-1',
    handlerSessionId: 'handler-session-demo-1',
    originalHandlerInvocationId: 'handler-invocation-demo-1',
    actionHandlerInvocationId: 'handler-action-demo-1',
    reviewInvocationId: 'review-invocation-demo-1',
    reviewHarnessRevisionId: 'review-revision-demo-1',
    reviewHarnessConfigurationDigest: 'review-digest-demo-1',
    reviewHarnessRepositoryCommitRef: 'accepted-checkpoint-55cdd40',
    deliveryRequestedAt: '2026-08-04T08:00:12Z',
    deliveryPersistedAt: '2026-08-04T08:00:12Z',
    harnessBoundAt: '2026-08-04T08:00:13Z',
    launchRequestedAt: '2026-08-04T08:00:14Z',
    launchAcceptedAt: '2026-08-04T08:00:15Z',
    reviewReadyAt: '2026-08-04T08:00:16Z',
    delivered: {
      summaryClaim: 'Implemented the bounded orchestration capability.',
      validationStatementClaim: 'Focused deterministic checks passed.',
      changedFiles: [
        {
          evidenceRef: 'evidence-demo-1',
          displayName: 'src/orchestration-capability.ts',
          changeKind: 'modified',
          contentFingerprint: 'content-demo-1',
        },
      ],
      comparisonFingerprint: 'comparison-demo-1',
      deliveredPayloadFingerprint: 'delivery-demo-1',
    },
    ...(state === 'accepted'
      ? {
          semanticJudgment: {
            variant: 'accept',
            fingerprint: 'judgment-demo-1',
            recordedAt: '2026-08-04T08:00:17Z',
          },
          lifecycle: { status: 'completed', observedAt: '2026-08-04T08:00:18Z' },
        }
      : {}),
  };
}

function handlerDecision(): ProductWorkUnitHandlerDecisionV1 {
  return {
    reviewInvocationId: 'review-invocation-demo-1',
    variant: 'accepted',
    fingerprint: 'decision-demo-1',
    recordedAt: '2026-08-04T08:00:19Z',
    implementationAcceptedAt: '2026-08-04T08:00:19Z',
  };
}

function integrationResult(): ProductWorkUnitIntegrationV1 {
  return {
    requestedAt: '2026-08-04T08:00:20Z',
    authorizedAt: '2026-08-04T08:00:20Z',
    progress: { phase: 'recording', recordedAt: '2026-08-04T08:00:24Z' },
    success: { recordedAt: '2026-08-04T08:00:25Z' },
    settlement: { settledAt: '2026-08-04T08:00:25Z' },
    prerequisiteContribution: {
      recordedAt: '2026-08-04T08:00:25Z',
      dependentCount: 2,
    },
  };
}
