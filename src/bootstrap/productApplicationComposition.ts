import { tauriAgentSessionImportClient } from '../infrastructure/agentSessions/tauriAgentSessionImportClient';
import { tauriSessionNavigationAgent } from '../infrastructure/agentSessions/tauriSessionNavigationAgent';
import { tauriSessionNavigationClient } from '../infrastructure/agentSessions/tauriSessionNavigationClient';
import { tauriSessionDeepLinks } from '../infrastructure/agentSessions/tauriSessionDeepLinks';
import { tauriExecutionTargetClient } from '../infrastructure/executionTargets/tauriExecutionTargetClient';
import { worktreeReviewBranchSource } from '../infrastructure/branches/worktreeReviewBranchSource';
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
import { tauriOtpCatalogueReader, tauriOtpInstallationClient } from '../infrastructure/otp';
import { tauriWorkflowInstanceClient } from '../infrastructure/workflowInstances/tauriWorkflowInstanceClient';
import { tauriDraftCloseGuard } from '../infrastructure/tauriDraftCloseGuard';
import { tauriSessionEventQueryClient } from '../infrastructure/sessionEvents/tauriSessionEventQueryClient';
import { tauriNativeProfileClient } from '../infrastructure/nativeProfiles/nativeProfileClient';
import { createNativeProfileApplicationConsumer } from '../infrastructure/nativeProfiles/nativeProfileConsumer';
import {
  tauriProductDecisionClient,
  tauriProductDecisionCorrectionClient,
} from '../infrastructure/productDecisions/tauriProductDecisionClient';
import { createRepositoryWorktreeTargetSelector } from '../features/repositoryCatalog';
import { tauriWorktreeReview } from '../infrastructure/tauriWorktreeReview';
import { tauriRepositoryCatalog } from '../infrastructure/repositoryCatalog/tauriRepositoryCatalog';

const RepositoryWorktreeTargetSelector =
  createRepositoryWorktreeTargetSelector(tauriRepositoryCatalog);

/** Product boot owns only available application boundaries; absent orchestration runtime stays explicit. */
export function createProductApplicationComposition(): AppProps {
  return {
    agentSessionClient: tauriAgentSessionClient,
    agentSessionImportClient: tauriAgentSessionImportClient,
    sessionNavigationClient: tauriSessionNavigationClient,
    sessionDeepLinks: tauriSessionDeepLinks,
    sessionNavigationAgent: tauriSessionNavigationAgent,
    workflowAuthoringClient: tauriWorkflowAuthoringClient,
    otpCatalogueReader: tauriOtpCatalogueReader,
    otpInstallationClient: tauriOtpInstallationClient,
    workflowInstanceClient: tauriWorkflowInstanceClient,
    draftCloseGuard: tauriDraftCloseGuard,
    executionConfigurationClient: tauriExecutionConfigurationClient,
    executionTargetClient: tauriExecutionTargetClient,
    branchSource: worktreeReviewBranchSource(tauriWorktreeReview),
    identityManagementClient: tauriIdentityManagementClient,
    agentSessionProfileClient: tauriAgentSessionClient,
    sessionEventQueryClient: tauriSessionEventQueryClient,
    workflowTargetSelector: RepositoryWorktreeTargetSelector,
    repositoryCatalogClient: tauriRepositoryCatalog,
    worktreeReviewClient: tauriWorktreeReview,
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
