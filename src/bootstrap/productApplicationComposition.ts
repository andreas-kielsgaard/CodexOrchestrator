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
import { createTauriContextualFileReviewClient } from '../infrastructure/fileReview/tauriContextualFileReview';
import { tauriIdentityManagementClient } from '../infrastructure/identities';
import { tauriExecutionConfigurationClient } from '../infrastructure/executionConfiguration/tauriExecutionConfigurationClient';
import { tauriWorkflowAuthoringClient } from '../infrastructure/workflowAuthoring/tauriWorkflowAuthoringClient';
import { tauriAgentSessionProfileClient } from '../infrastructure/agentSessionProfiles/tauriAgentSessionProfileClient';
import { tauriSessionEventQueryClient } from '../infrastructure/sessionEvents/tauriSessionEventQueryClient';
import { tauriNativeProfileClient } from '../infrastructure/nativeProfiles/nativeProfileClient';
import { createNativeProfileApplicationConsumer } from '../infrastructure/nativeProfiles/nativeProfileConsumer';
import {
  tauriProductDecisionClient,
  tauriProductDecisionCorrectionClient,
} from '../infrastructure/productDecisions/tauriProductDecisionClient';
import { DiscoveredWorktreeTargetSelector } from '../features/worktreeTargetsTemp/DiscoveredWorktreeTargetSelector';

/** Product boot owns only available application boundaries; absent orchestration runtime stays explicit. */
export function createProductApplicationComposition(): AppProps {
  return {
    agentSessionClient: tauriAgentSessionClient,
    workflowAuthoringClient: tauriWorkflowAuthoringClient,
    executionConfigurationClient: tauriExecutionConfigurationClient,
    identityManagementClient: tauriIdentityManagementClient,
    agentSessionProfileClient: tauriAgentSessionProfileClient,
    sessionEventQueryClient: tauriSessionEventQueryClient,
    workflowTargetSelector: DiscoveredWorktreeTargetSelector,
    managedPlanBuilderSessionClient: createTauriManagedPlanBuilderSessionClient(
      tauriAgentSessionClient,
      invoke,
    ),
    contextualFileReviewClient: createTauriContextualFileReviewClient(),
    nativeProfileClient: tauriNativeProfileClient,
    nativeProfileApplicationConsumer:
      createNativeProfileApplicationConsumer(tauriNativeProfileClient),
    productDecisionClient: tauriProductDecisionClient,
    productDecisionCorrectionClient: tauriProductDecisionCorrectionClient,
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
