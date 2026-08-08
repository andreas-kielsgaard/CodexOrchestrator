import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  EPIC_INITIATION_CONFIRMATION_EVENT,
  EpicInitiationConfirmationError,
  confirmationFailureKind,
  decodeEpicInitiationConfirmationEvent,
  decodeEpicInitiationConfirmationRequest,
  decodeEpicInitiationConfirmationResolution,
  type EpicInitiationConfirmationClient,
} from '../../application/orchestrations';
import type { OrchestrationNativeQueryClient } from './tauriOrchestrationNativeQuery';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type TauriListen = (
  event: string,
  handler: (event: { payload: unknown }) => void,
) => Promise<UnlistenFn>;

export function createTauriEpicInitiationConfirmationClient(
  nativeQuery: OrchestrationNativeQueryClient,
  invokeCommand: TauriInvoke = invoke,
  listenEvent: TauriListen = listen,
): EpicInitiationConfirmationClient {
  return {
    async request(input) {
      try {
        return decodeEpicInitiationConfirmationRequest(
          await invokeCommand('request_epic_initiation_confirmation', { input }),
        );
      } catch (error) {
        if (error instanceof Error && error.message.startsWith('Invalid Epic initiation'))
          throw error;
        throw new EpicInitiationConfirmationError(confirmationFailureKind(error));
      }
    },
    async resolve(requestId, decision, rootBranch) {
      try {
        return decodeEpicInitiationConfirmationResolution(
          await invokeCommand('resolve_epic_initiation_confirmation', {
            input: { requestId, decision, ...(rootBranch ? { rootBranch } : {}) },
          }),
        );
      } catch (error) {
        if (error instanceof Error && error.message.startsWith('Invalid Epic initiation'))
          throw error;
        throw new EpicInitiationConfirmationError(confirmationFailureKind(error));
      }
    },
    subscribe(listener, onMalformed) {
      return listenEvent(EPIC_INITIATION_CONFIRMATION_EVENT, (event) => {
        try {
          listener(decodeEpicInitiationConfirmationEvent(event.payload));
        } catch {
          onMalformed();
        }
      });
    },
    async describe(request) {
      const query = await nativeQuery.load();
      const draft = query.planningDrafts.find(
        (item) => item.epicPlanningDraftId === request.epicPlanningDraftId,
      );
      if (!draft || draft.currentProposal.status !== 'available')
        throw new Error('Current proposal is unavailable.');
      const proposalRevisionId = draft.currentProposal.proposalRevisionId;
      const revision = query.proposalRevisions.find(
        (item) => item.proposalRevisionId === proposalRevisionId,
      );
      if (!revision) throw new Error('Current proposal revision is unavailable.');
      return {
        title: revision.proposal.suggestedEpicName ?? draft.title ?? 'Untitled Epic',
        sprintTitles: revision.proposal.sprints.map((sprint) => sprint.title),
      };
    },
  };
}
