import { describe, expect, it } from 'vitest';
import { createTauriProductDecisionClient } from './tauriProductDecisionClient';

describe('durable Product Decision transport', () => {
  it('uses only the productive current/history/explicit-acceptance commands', async () => {
    const calls: string[] = [];
    const client = createTauriProductDecisionClient(async <T>(command: string) => {
      calls.push(command);
      return (command === 'load_product_decision_current_query' ? { decisions: [] } : []) as T;
    });
    await expect(client.loadCurrent()).resolves.toEqual([]);
    await client.loadHistory('epic', 'decision');
    await client.acceptVersion({
      decisionId: 'decision',
      epicId: 'epic',
      idempotencyKey: 'key',
      title: 't',
      statement: 's',
      intent: 'i',
      acceptanceProvenance: { kind: 'manual_human_application' },
    });
    expect(calls).toEqual([
      'load_product_decision_current_query',
      'load_product_decision_history',
      'accept_product_decision_version',
    ]);
  });
});
