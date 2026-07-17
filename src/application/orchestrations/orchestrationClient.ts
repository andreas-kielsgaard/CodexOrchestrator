import { composeProductOrchestrationReadModels } from './productReadModelComposer';
import type { ProductReadCompositionInputV1, ProductReadModelsV1 } from './productReadModels';

/** Provider-neutral application read boundary. It owns loading, never execution or persistence. */
export interface OrchestrationApplicationClient {
  load(): Promise<OrchestrationLoadResult>;
}

export type OrchestrationLoadResult =
  | { readonly kind: 'ready'; readonly readModels: ProductReadModelsV1 }
  | { readonly kind: 'empty'; readonly reason: string }
  | { readonly kind: 'unavailable'; readonly reason: string }
  | { readonly kind: 'failed'; readonly message: string };

/** Builds recorded reads through the same canonical product composition boundary as a product source. */
export function recordedOrchestrationClient(
  input: ProductReadCompositionInputV1,
): OrchestrationApplicationClient {
  return {
    async load() {
      return { kind: 'ready', readModels: composeProductOrchestrationReadModels(input) };
    },
  };
}

/** No durable product connector exists yet; absence is an explicit normal result. */
export const unavailableProductOrchestrationClient: OrchestrationApplicationClient = {
  async load() {
    return {
      kind: 'unavailable',
      reason: 'No durable orchestration data source is connected to this application yet.',
    };
  },
};
