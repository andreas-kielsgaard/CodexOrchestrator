import { invoke } from '@tauri-apps/api/core';
import type {
  AcceptProductDecisionVersionInput,
  ProductDecisionClient,
  ProductDecisionCurrent,
  ProductDecisionVersion,
} from '../../application/productDecisions';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

/** Product transport for durable decisions; it never exposes the recorded-development adapter. */
export function createTauriProductDecisionClient(
  invokeCommand: Invoke = invoke,
): ProductDecisionClient {
  return {
    async loadCurrent() {
      const result = await invokeCommand<{ decisions: ProductDecisionCurrent[] }>(
        'load_product_decision_current_query',
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
