import { invoke } from '@tauri-apps/api/core';
import type {
  AddOrchestrationDraftNoteInput,
  AttachOrchestrationDraftFilesInput,
  CreateOrchestrationDraftInput,
  OrchestrationBuildPackage,
  OrchestrationClient,
  OrchestrationRegistrySnapshot,
  OrchestrationSnapshot,
  RequestOrchestrationBuildStageInput,
  StartOrchestrationPlanBuilderRunInput,
  StartOrchestrationInput,
  StartOrchestrationResult,
} from '../application/orchestrationClient';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriOrchestrationClient(
  invokeCommand: TauriInvoke = invoke,
): OrchestrationClient {
  return {
    loadOrchestrations(): Promise<OrchestrationRegistrySnapshot> {
      return invokeCommand<OrchestrationRegistrySnapshot>('load_orchestration_registry');
    },
    createDraft(input: CreateOrchestrationDraftInput): Promise<OrchestrationBuildPackage> {
      return invokeCommand<OrchestrationBuildPackage>('create_orchestration_draft', { input });
    },
    addDraftNote(input: AddOrchestrationDraftNoteInput): Promise<OrchestrationBuildPackage> {
      return invokeCommand<OrchestrationBuildPackage>('add_orchestration_draft_note', { input });
    },
    attachDraftFiles(
      input: AttachOrchestrationDraftFilesInput,
    ): Promise<OrchestrationBuildPackage> {
      return invokeCommand<OrchestrationBuildPackage>('attach_orchestration_draft_files', {
        input,
      });
    },
    requestBuildStage(
      input: RequestOrchestrationBuildStageInput,
    ): Promise<OrchestrationBuildPackage> {
      return invokeCommand<OrchestrationBuildPackage>('request_orchestration_build_stage', {
        input,
      });
    },
    startPlanBuilderRun(
      input: StartOrchestrationPlanBuilderRunInput,
    ): Promise<OrchestrationBuildPackage> {
      return invokeCommand<OrchestrationBuildPackage>('start_orchestration_plan_builder_run', {
        input,
      });
    },
    startOrchestration(input: StartOrchestrationInput): Promise<StartOrchestrationResult> {
      return invokeCommand<StartOrchestrationResult>('start_orchestration', { input });
    },
    loadOrchestration(id: string): Promise<OrchestrationSnapshot | null> {
      return invokeCommand<OrchestrationSnapshot | null>('load_orchestration', { id });
    },
    cancelDraft(buildPackageId: string): Promise<OrchestrationRegistrySnapshot> {
      return invokeCommand<OrchestrationRegistrySnapshot>('cancel_orchestration_draft', {
        buildPackageId,
      });
    },
  };
}

export const tauriOrchestrationClient = createTauriOrchestrationClient();
