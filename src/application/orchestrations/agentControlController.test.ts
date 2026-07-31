import { describe, expect, it, vi } from 'vitest';
import {
  recordedAgentControlController,
  unsupportedProductSprintAgentControlController,
  unsupportedProductEpicAgentControlController,
  type ContinuationIntent,
} from './agentControlController';
import { composeProductOrchestrationReadModels } from './productReadModelComposer';
import { recordedProductReadCompositionInput } from '../../dev/orchestrationSection/recordedProductReadCompositionInput';

const command = {
  agentControlCommandId: 'command-1',
  commandKind: 'request_next_work_slice_planner' as const,
  recipientAgentSessionRefId: 'session-ref-1',
  target: { kind: 'next_work_slice_planner' as const, sprintId: 'sprint-control-surface' },
  idempotency: { key: 'key-1', scopeKind: 'sprint' as const, scopeId: 'sprint-control-surface' },
  initiatedBy: { sourceKind: 'user_authored' as const, sourceReference: 'prompt-1' },
  promptProvenanceId: 'prompt-1',
  recordedAt: '2026-07-15T00:00:00.000Z',
  preconditionEvidenceReference: 'eligibility-sprint',
  continuation: {
    policyId: 'recorded-sprint-policy',
    eligibilityEvaluationId: 'eligibility-sprint',
  },
};
const recorded = {
  agentControlResultId: 'recorded-result',
  agentControlCommandId: command.agentControlCommandId,
  state: 'orchestration_event_recorded' as const,
  orchestrationEventReference: 'recorded-event',
  recordedAt: '2026-07-15T00:00:00.000Z',
};

describe('Agent Control controllers', () => {
  it('keeps product Sprint and Epic boundaries explicitly unsupported', async () => {
    await expect(
      unsupportedProductSprintAgentControlController.requestContinuation(sprintIntent()),
    ).resolves.toMatchObject({ status: 'unsupported' });
    await expect(
      unsupportedProductEpicAgentControlController.requestContinuation(epicIntent()),
    ).resolves.toMatchObject({ status: 'unsupported' });
  });

  it('materializes separate canonical intents outside JSX and rejects an unrefreshed event result', async () => {
    const materialize = vi.fn(async () => command);
    const controller = recordedAgentControlController(
      async () => validCanonicalHistoricalInput(),
      async () => recorded,
      materialize,
    );
    const outcome = await controller.requestContinuation(sprintIntent());
    expect(materialize).toHaveBeenCalledWith(sprintIntent());
    expect(outcome.status).toBe('failed');
  });

  it.each([
    { ...recorded, agentControlCommandId: 'contradictory-command' },
    { ...recorded, orchestrationEventReference: 'contradictory-event' },
    { ...recorded, recordedAt: '2026-01-01T00:00:00.000Z' },
  ])('rejects same-id returned results with contradictory causal fields', async (returned) => {
    const controller = recordedAgentControlController(
      async () => validCanonicalHistoricalInput(),
      async () => returned,
      async () => command,
    );
    await expect(controller.requestContinuation(sprintIntent())).resolves.toMatchObject({
      status: 'failed',
    });
  });

  it('rejects an otherwise canonical historical result for a different submitted command', async () => {
    const canonicalInput = validCanonicalHistoricalInput();
    const historicalResult = canonicalInput.agentControl.results[0]!;
    expect(() => composeProductOrchestrationReadModels(canonicalInput)).not.toThrow();
    const refresh = vi.fn(async () => canonicalInput);
    const controller = recordedAgentControlController(
      refresh,
      async () => historicalResult,
      async () => command,
    );

    await expect(controller.requestContinuation(sprintIntent())).resolves.toMatchObject({
      status: 'failed',
      message: 'Continuation result did not belong to the submitted command.',
    });
    expect(refresh).not.toHaveBeenCalled();
  });
});

function sprintIntent(): Extract<ContinuationIntent, { readonly level: 'sprint' }> {
  return {
    level: 'sprint',
    sprintId: 'sprint-control-surface',
    policyId: 'recorded-sprint-policy',
    eligibilityEvaluationId: 'eligibility-sprint',
  };
}

function validCanonicalHistoricalInput() {
  const input = structuredClone(recordedProductReadCompositionInput);
  const promptProvenance = {
    promptProvenanceId: 'historical-prompt',
    sourceKind: 'application_produced',
    sourceReference: 'historical-agent-control-test',
    causalInputReferences: [],
  } as const;
  const historicalCommand = {
    agentControlCommandId: 'historical-command',
    commandKind: 'request_agent_session_prompt',
    recipientAgentSessionRefId: 'session-ref-epic-runner',
    target: { kind: 'agent_session', agentSessionRefId: 'session-ref-epic-runner' },
    idempotency: {
      key: 'historical-command-key',
      scopeKind: 'agent_session',
      scopeId: 'session-ref-epic-runner',
    },
    initiatedBy: {
      sourceKind: 'application_produced',
      sourceReference: 'historical-agent-control-test',
    },
    promptProvenanceId: 'historical-prompt',
    recordedAt: '2026-07-15T00:00:00.000Z',
    preconditionEvidenceReference: 'historical-precondition',
  } as const;
  const historicalResult = {
    agentControlResultId: 'historical-result',
    agentControlCommandId: 'historical-command',
    state: 'orchestration_event_recorded',
    orchestrationEventReference: 'completion-sprint-control-surface',
    recordedAt: '2026-07-15T00:00:00.000Z',
  } as const;
  return {
    ...input,
    agentControl: {
      ...input.agentControl,
      promptProvenance: [...input.agentControl.promptProvenance, promptProvenance],
      commands: [...input.agentControl.commands, historicalCommand],
      results: [...input.agentControl.results, historicalResult],
    },
  };
}

function epicIntent(): Extract<ContinuationIntent, { readonly level: 'epic' }> {
  return {
    level: 'epic',
    epicId: 'recorded-epic',
    policyId: 'recorded-epic-policy',
    eligibilityEvaluationId: 'eligibility-epic',
  };
}
