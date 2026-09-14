import { recordedOrchestrationClient } from '../../application/orchestrations';
import {
  presentProductOrchestrations,
  type OrchestrationPresentationAdapter,
} from '../../app/orchestrationPresentation';
import {
  recordedAgentSessionDetails,
  recordedPresentationAdjunct,
} from './recordedPresentationAdjunct';
import { recordedProductReadCompositionInput } from './recordedProductReadCompositionInput';
import {
  unsupportedArtifactAccessController,
  unsupportedProductSprintAutomaticContinuationPolicyController,
  unsupportedProductEpicAutomaticContinuationPolicyController,
} from '../../application/orchestrations';
import type { AppProps } from '../../app/App';
import type { AgentIdentity, AgentSessionClient } from '../../application/agentSessions';
import { recordedLocalEpicPlanProposalSource } from './recordedEpicPlanProposalSource';
import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorAgentIdentity,
  recordedHarnessInspectorSessionDetails,
  recordedHarnessInspectorSessionId,
} from '../conversationHarnesses/recordedHarnessInspectorSource';
import { HarnessInspectorDevelopmentSurface } from '../../features/conversationHarnesses';
import { createElement } from 'react';
import {
  addRecordedWorkUnitReviewInspection,
  recordedWorkUnitReviewFileSource,
} from './recordedWorkUnitReviewFixture';
import { recordedEpicProductDecisionSource } from '../productDecisions/recordedEpicProductDecisionSource';

/** Recorded development data enters through canonical composition; it is not a product connector. */
export const recordedDevelopmentOrchestrationClient = recordedOrchestrationClient(
  recordedProductReadCompositionInput,
);

const previewSessions = [...recordedAgentSessionDetails, recordedHarnessInspectorSessionDetails];

const loadPreviewSession: AgentSessionClient['loadSession'] = async ({ sessionId }) => {
  const details = previewSessions.find(({ session }) => session.id === sessionId);
  if (!details) throw new Error(`Recorded preview Session not found: ${sessionId}`);
  return details;
};

const unsupportedSessionMutation = async (): Promise<never> => {
  throw new Error(
    'Recorded previews support Session inspection only. Session changes are unavailable.',
  );
};

export const recordedDevelopmentAgentSessionClient: AgentSessionClient = {
  listSessions: async ({ availability, limit } = {}) =>
    previewSessions
      .filter(({ session }) => !availability || session.availability === availability)
      .slice(0, limit)
      .map(({ session, invocations }) => ({
        id: session.id,
        title: session.title,
        availability: session.availability,
        pendingRequestCount: 0,
        hasActiveInvocation: invocations.some(({ invocation }) =>
          ['pending', 'running'].includes(invocation.status),
        ),
        latestInvocationStatus: invocations.at(-1)?.invocation.status ?? null,
        createdAt: session.createdAt,
        updatedAt: session.updatedAt,
      })),
  loadSession: loadPreviewSession,
  reloadSession: loadPreviewSession,
  subscribeUpdates: async () => () => undefined,
  disconnectUpdates: async () => undefined,
  createSession: unsupportedSessionMutation,
  sendMessage: unsupportedSessionMutation,
  cancelInvocation: unsupportedSessionMutation,
};

/** Stable development identity for visual review; it is not a durable product assignment. */
export const recordedPlanBuilderAgentIdentity: AgentIdentity = {
  name: 'Avery',
  harnessRole: 'epic_plan_builder',
  visualIdentityToken: 'sunflower',
};

/** Compatibility-only transcripts and workflow geometry are adjuncts, never product read facts. */
export function createRecordedDevelopmentOrchestrationPresentation(options?: {
  readonly includeWorkUnitReview?: boolean;
}): OrchestrationPresentationAdapter {
  return {
    present(readModels) {
      const presentationReads = options?.includeWorkUnitReview
        ? addRecordedWorkUnitReviewInspection(readModels)
        : readModels;
      const product = presentationReads.epics[0];
      if (!product) throw new Error('Recorded adjunct requires a canonical orchestration read.');
      for (const sprintId of Object.keys(recordedPresentationAdjunct.sprints ?? {})) {
        if (!product.sprints.some((sprint) => sprint.sprintId === sprintId))
          throw new Error(`Recorded adjunct has no canonical Sprint read for ${sprintId}.`);
      }
      return presentProductOrchestrations(presentationReads, recordedPresentationAdjunct);
    },
  };
}

export const recordedDevelopmentOrchestrationPresentation =
  createRecordedDevelopmentOrchestrationPresentation();

/** Development-only adapter: recorded reads use the product tree, while effects remain unsupported. */
export function createRecordedDevelopmentApplicationComposition(options?: {
  readonly initialSurface?: AppProps['initialSurface'];
  readonly includeWorkUnitReview?: boolean;
}): AppProps {
  let harnessIdentity = recordedHarnessInspectorAgentIdentity;
  const harnessManagementSource = createRecordedHarnessManagementSource({
    onSessionIdentityChange(identity) {
      harnessIdentity = identity;
    },
  });
  return {
    agentSessionClient: recordedDevelopmentAgentSessionClient,
    managedPlanBuilderAgentIdentity: recordedPlanBuilderAgentIdentity,
    orchestrationClient: recordedDevelopmentOrchestrationClient,
    orchestrationPresentation: createRecordedDevelopmentOrchestrationPresentation({
      includeWorkUnitReview: options?.includeWorkUnitReview,
    }),
    orchestrationAgentSessionComposition: { client: recordedDevelopmentAgentSessionClient },
    fileReviewSourceForEvidence: (target) =>
      recordedWorkUnitReviewFileSource(target.reviewId, target.changedFileId),
    epicProductDecisionSource: recordedEpicProductDecisionSource,
    artifactAccessController: unsupportedArtifactAccessController,
    sprintAutomaticContinuationPolicyController:
      unsupportedProductSprintAutomaticContinuationPolicyController,
    epicAutomaticContinuationPolicyController:
      unsupportedProductEpicAutomaticContinuationPolicyController,
    epicPlanProposalSource: recordedLocalEpicPlanProposalSource,
    agentSessionHarnessManagementSource: harnessManagementSource,
    agentIdentityForSession: (sessionId) =>
      sessionId === recordedHarnessInspectorSessionId ? harnessIdentity : undefined,
    harnessManagementPreviewSurface: createElement(HarnessInspectorDevelopmentSurface, {
      composition: {
        client: recordedDevelopmentAgentSessionClient,
        sessionId: recordedHarnessInspectorSessionId,
        source: harnessManagementSource,
        agentIdentity: harnessIdentity,
      },
    }),
    initialSurface: options?.initialSurface,
  };
}
