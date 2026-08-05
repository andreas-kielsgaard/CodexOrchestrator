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
import type { AgentIdentity } from '../../application/agentSessions';
import {
  createRecordedAgentSessionClient,
  createRecordedAgentSessionStore,
} from '../agentSessions';
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

/** Recorded development data enters through canonical composition; it is not a product connector. */
export const recordedDevelopmentOrchestrationClient = recordedOrchestrationClient(
  recordedProductReadCompositionInput,
);

/** Deterministic Agent Session client for the same embedded component tree in recorded mode. */
export const recordedDevelopmentAgentSessionClient = createRecordedAgentSessionClient({
  store: createRecordedAgentSessionStore([
    ...recordedAgentSessionDetails,
    recordedHarnessInspectorSessionDetails,
  ]),
});

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
