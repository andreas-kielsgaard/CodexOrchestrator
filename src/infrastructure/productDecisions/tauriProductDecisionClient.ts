import { invoke } from '@tauri-apps/api/core';
import type {
  AcceptProductDecisionVersionInput,
  ProductDecisionClient,
  ProductDecisionCorrectionClient,
  ProductDecisionCorrectionConversation,
  ProductDecisionCorrectionProposal,
  ProductDecisionCurrent,
  ProductDecisionVersion,
} from '../../application/productDecisions';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

/** Product transport for durable decisions; it never exposes the recorded-development adapter. */
export function createTauriProductDecisionClient(
  invokeCommand: Invoke = invoke,
): ProductDecisionClient {
  return {
    async loadCurrent(epicId) {
      const result = await invokeCommand<{ decisions: ProductDecisionCurrent[] }>(
        'load_product_decision_current_query',
        { input: { epicId } },
      );
      return result.decisions;
    },
    loadHistory(epicId, decisionId) {
      return invokeCommand<ProductDecisionVersion[]>('load_product_decision_history', {
        input: { epicId, decisionId },
      });
    },
    acceptVersion(input: AcceptProductDecisionVersionInput) {
      return invokeCommand<ProductDecisionVersion>('accept_product_decision_version', { input });
    },
  };
}

export const tauriProductDecisionClient = createTauriProductDecisionClient();

/** Dedicated transport for the Product Decision-owned correction boundary. */
export function createTauriProductDecisionCorrectionClient(
  invokeCommand: Invoke = invoke,
): ProductDecisionCorrectionClient {
  return {
    startConversation(input) {
      return invokeCommand<ProductDecisionCorrectionConversation>(
        'start_product_decision_correction_conversation',
        { input },
      );
    },
    sendMessage(input) {
      return invokeCommand<Readonly<{ sessionId: string; invocationId: string }>>(
        'send_product_decision_correction_message',
        { input },
      );
    },
    saveProposal(input) {
      return invokeCommand<ProductDecisionCorrectionProposal>(
        'save_product_decision_correction_proposal',
        { input },
      );
    },
    acceptProposal(input) {
      return invokeCommand<ProductDecisionVersion>('accept_product_decision_correction_proposal', {
        input,
      });
    },
  };
}

export const tauriProductDecisionCorrectionClient = createTauriProductDecisionCorrectionClient();
