import { composeProductOrchestrationReadModels } from './productReadModelComposer';
import type { AgentControlCommandV1, AgentControlResultV1 } from './agentControl';
import type { ProductReadCompositionInputV1, ProductReadModelsV1 } from './productReadModels';

export type AgentControlCommandOutcome =
  | { readonly status: 'unsupported' | 'denied_ineligible' | 'failed'; readonly message: string }
  | {
      readonly status: 'orchestration_event_recorded';
      readonly result: AgentControlResultV1;
      readonly refreshedReadModels: ProductReadModelsV1;
    };

/** Canonical context supplied by presentation; command materialization stays application-owned. */
export type ContinuationIntent =
  | {
      readonly level: 'sprint';
      readonly sprintId: string;
      readonly policyId: string;
      readonly eligibilityEvaluationId: string;
    }
  | {
      readonly level: 'epic';
      readonly epicId: string;
      readonly policyId: string;
      readonly eligibilityEvaluationId: string;
    };

export interface SprintAgentControlController {
  requestContinuation(
    intent: Extract<ContinuationIntent, { readonly level: 'sprint' }>,
  ): Promise<AgentControlCommandOutcome>;
}

export interface EpicAgentControlController {
  requestContinuation(
    intent: Extract<ContinuationIntent, { readonly level: 'epic' }>,
  ): Promise<AgentControlCommandOutcome>;
}

/** Honest product boundary until a durable command handler is connected. */
export const unsupportedProductSprintAgentControlController: SprintAgentControlController = {
  async requestContinuation() {
    return {
      status: 'unsupported',
      message: 'Sprint continuation is not connected to a durable application handler.',
    };
  },
};

/** Honest product boundary until a durable command handler is connected. */
export const unsupportedProductEpicAgentControlController: EpicAgentControlController = {
  async requestContinuation() {
    return {
      status: 'unsupported',
      message: 'Epic continuation is not connected to a durable application handler.',
    };
  },
};

/**
 * Recorded-only adapter for deterministic development fixtures. An event-recorded result is
 * accepted only after re-composing the canonical read from refreshed input.
 */
export function recordedAgentControlController(
  refresh: () => Promise<ProductReadCompositionInputV1>,
  handle: (command: AgentControlCommandV1) => Promise<AgentControlResultV1>,
  materialize: (intent: ContinuationIntent) => Promise<AgentControlCommandV1>,
): SprintAgentControlController & EpicAgentControlController {
  return {
    async requestContinuation(intent) {
      const command = await materialize(intent);
      const result = await handle(command);
      if (result.agentControlCommandId !== command.agentControlCommandId)
        return {
          status: 'failed',
          message: 'Continuation result did not belong to the submitted command.',
        };
      if (result.state !== 'orchestration_event_recorded') {
        if (
          result.state === 'denied_ineligible' ||
          result.state === 'failed' ||
          result.state === 'unsupported'
        )
          return {
            status: result.state,
            message: `Continuation command ${result.state.replace('_', ' ')}.`,
          };
        return { status: 'failed', message: 'Continuation did not record an Orchestration Event.' };
      }
      const input = await refresh();
      let refreshedReadModels: ProductReadModelsV1;
      try {
        refreshedReadModels = composeProductOrchestrationReadModels(input);
      } catch {
        return {
          status: 'failed',
          message: 'Recorded continuation result contradicted refreshed canonical events.',
        };
      }
      const canonicalResult = input.agentControl.results.find(
        (candidate) => candidate.agentControlResultId === result.agentControlResultId,
      );
      if (
        !canonicalResult ||
        canonicalResult.agentControlCommandId !== result.agentControlCommandId ||
        canonicalResult.state !== 'orchestration_event_recorded' ||
        !canonicalResult.orchestrationEventReference ||
        canonicalResult.orchestrationEventReference !== result.orchestrationEventReference ||
        canonicalResult.recordedAt !== result.recordedAt
      ) {
        return {
          status: 'failed',
          message: 'Recorded continuation result was not present in refreshed canonical events.',
        };
      }
      return {
        status: 'orchestration_event_recorded',
        result: canonicalResult,
        refreshedReadModels,
      };
    },
  };
}
