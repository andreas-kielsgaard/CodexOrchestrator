import {
  createEpicInitiationCapability,
  unsupportedArtifactAccessController,
  unsupportedProductSprintAutomaticContinuationPolicyController,
  unsupportedProductEpicAutomaticContinuationPolicyController,
} from '../application/orchestrations';
import type { AppProps } from '../app/App';
import { tauriAgentSessionClient } from '../infrastructure/agentSessions/tauriAgentSessionClient';
import { invoke } from '@tauri-apps/api/core';
import { createTauriManagedPlanBuilderSessionClient } from '../infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient';
import {
  createNativeEpicPlanProposalSource,
  createNativeQueryOrchestrationClient,
  tauriOrchestrationNativeQueryClient,
} from '../infrastructure/orchestrations/tauriOrchestrationNativeQuery';
import { createTauriEpicPlanningDraftLifecycleClient } from '../infrastructure/orchestrations/tauriEpicPlanningDraftLifecycle';
import { createTauriEpicInitiationConfirmationClient } from '../infrastructure/orchestrations/tauriEpicInitiationConfirmation';
import { tauriEpicBootstrapTransitionClient } from '../infrastructure/orchestrations/tauriEpicBootstrapTransition';
import { tauriSprintRunnerTransitionClient } from '../infrastructure/orchestrations/tauriSprintRunnerTransition';
import { createTauriConversationHarnessInspectorSource } from '../infrastructure/conversationHarnesses/tauriConversationHarnessInspectorSource';
import { createTauriContextualFileReviewClient } from '../infrastructure/fileReview/tauriContextualFileReview';
import { tauriNativeProfileClient } from '../infrastructure/nativeProfiles/nativeProfileClient';
import { tauriEpicOriginProjectClient } from '../infrastructure/epicOriginProject/tauriEpicOriginProject';
import { createNativeProfileApplicationConsumer } from '../infrastructure/nativeProfiles/nativeProfileConsumer';
import {
  tauriProductDecisionClient,
  tauriProductDecisionCorrectionClient,
} from '../infrastructure/productDecisions/tauriProductDecisionClient';
import { tauriWorktreeReviewClient } from '../infrastructure/tauriWorktreeReview';

/** Product boot owns only available application boundaries; absent orchestration runtime stays explicit. */
export function createProductApplicationComposition(options?: {
  /** The launched review instance must not offer a launcher for another review instance. */
  readonly includeWorktreeReview?: boolean;
}): AppProps {
  return {
    agentSessionClient: tauriAgentSessionClient,
    epicOriginProjectClient: tauriEpicOriginProjectClient,
    managedPlanBuilderSessionClient: createTauriManagedPlanBuilderSessionClient(
      tauriAgentSessionClient,
      invoke,
    ),
    agentSessionHarnessManagementSource: createTauriConversationHarnessInspectorSource(invoke),
    contextualFileReviewClient: createTauriContextualFileReviewClient(),
    nativeProfileClient: tauriNativeProfileClient,
    nativeProfileApplicationConsumer:
      createNativeProfileApplicationConsumer(tauriNativeProfileClient),
    productDecisionClient: tauriProductDecisionClient,
    productDecisionCorrectionClient: tauriProductDecisionCorrectionClient,
    ...(options?.includeWorktreeReview === false
      ? {}
      : { worktreeReviewClient: tauriWorktreeReviewClient }),
    orchestrationClient: createNativeQueryOrchestrationClient(
      tauriOrchestrationNativeQueryClient,
      tauriEpicBootstrapTransitionClient,
      tauriSprintRunnerTransitionClient,
    ),
    epicInitiationConfirmationClient: createTauriEpicInitiationConfirmationClient(
      tauriOrchestrationNativeQueryClient,
      invoke,
    ),
    orchestrationAgentSessionComposition: { client: tauriAgentSessionClient },
    artifactAccessController: unsupportedArtifactAccessController,
    sprintAutomaticContinuationPolicyController:
      unsupportedProductSprintAutomaticContinuationPolicyController,
    epicAutomaticContinuationPolicyController:
      unsupportedProductEpicAutomaticContinuationPolicyController,
    epicPlanningDraftLifecycleClient: createTauriEpicPlanningDraftLifecycleClient(
      invoke,
      tauriOrchestrationNativeQueryClient,
    ),
    epicPlanProposalSourceForDraft: (draftId) =>
      createNativeEpicPlanProposalSource(tauriOrchestrationNativeQueryClient, draftId),
    epicInitiationCapabilityForDraft: async (draftId) =>
      createEpicInitiationCapability(await tauriOrchestrationNativeQueryClient.load(), draftId),
  };
}
