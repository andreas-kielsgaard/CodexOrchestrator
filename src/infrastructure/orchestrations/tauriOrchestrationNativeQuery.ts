import { invoke } from '@tauri-apps/api/core';
import {
  decodeOrchestrationNativeQueryV2,
  projectEpicPlanProposal,
  nativeQueryProductCompositionInputV2,
  type EpicPlanProposalSnapshot,
  type EpicPlanProposalSource,
  type OrchestrationApplicationClient,
  type OrchestrationNativeQueryV2,
} from '../../application/orchestrations';
import { composeProductOrchestrationReadModels } from '../../application/orchestrations';
import type { EpicBootstrapTransitionClient } from './tauriEpicBootstrapTransition';

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export interface OrchestrationNativeQueryClient {
  load(): Promise<OrchestrationNativeQueryV2>;
}

export function createTauriOrchestrationNativeQueryClient(
  invokeCommand: TauriInvoke = invoke,
): OrchestrationNativeQueryClient {
  return {
    async load() {
      return decodeOrchestrationNativeQueryV2(
        await invokeCommand<unknown>('load_orchestration_native_query'),
      );
    },
  };
}

/** V2 contains planning drafts, not accepted orchestration roots; no root is invented here. */
export function createNativeQueryOrchestrationClient(
  nativeQuery: OrchestrationNativeQueryClient,
  transitionClient?: EpicBootstrapTransitionClient,
): OrchestrationApplicationClient {
  return {
    async load() {
      try {
        const query = await nativeQuery.load();
        const transitionQuery = transitionClient ? await transitionClient.load() : undefined;
        if (query.initiatedEpics.length)
          return {
            kind: 'ready',
            readModels: composeProductOrchestrationReadModels(
              nativeQueryProductCompositionInputV2(query, transitionQuery),
            ),
          };
        return { kind: 'empty', reason: 'No accepted Epic orchestration has been recorded.' };
      } catch {
        return {
          kind: 'unavailable',
          reason: 'The durable orchestration query is unavailable.',
        };
      }
    },
  };
}

export function createNativeEpicPlanProposalSource(
  nativeQuery: OrchestrationNativeQueryClient,
  epicPlanningDraftId: string,
): EpicPlanProposalSource {
  let snapshot: EpicPlanProposalSnapshot = {
    kind: 'unavailable',
    reason: 'Epic Plan Proposal has not been loaded.',
  };
  const listeners = new Set<() => void>();
  return {
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async refresh() {
      let next: EpicPlanProposalSnapshot;
      try {
        next = projectEpicPlanProposal(await nativeQuery.load(), epicPlanningDraftId);
      } catch {
        next = {
          kind: 'unavailable',
          reason: 'The durable Epic Plan Proposal could not be refreshed.',
        };
      }
      if (JSON.stringify(next) !== JSON.stringify(snapshot)) {
        snapshot = next;
        for (const listener of listeners) listener();
      }
    },
  };
}

export const tauriOrchestrationNativeQueryClient = createTauriOrchestrationNativeQueryClient();
