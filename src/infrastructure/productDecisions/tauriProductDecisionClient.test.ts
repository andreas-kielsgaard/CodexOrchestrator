import { describe, expect, it } from 'vitest';
import { createTauriProductDecisionClient } from './tauriProductDecisionClient';

describe('durable Product Decision transport', () => {
  it('uses only the productive current/history/explicit-acceptance commands', async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const client = createTauriProductDecisionClient(async <T>(command: string, args) => {
      calls.push({ command, args });
      return (command === 'load_product_decision_current_query' ? { decisions: [] } : []) as T;
    });
    await expect(client.loadCurrent('epic')).resolves.toEqual([]);
    await client.loadHistory('epic', 'decision');
    await client.acceptVersion({
      decisionId: 'decision',
      epicId: 'epic',
      idempotencyKey: 'key',
      title: 't',
      statement: 's',
      intent: 'i',
      acceptanceProvenance: {
        kind: 'manual_human_application',
        humanInteractionOrigin: { kind: 'human_interaction', opaqueId: 'human-1' },
      },
    });
    expect(calls).toEqual([
      { command: 'load_product_decision_current_query', args: { input: { epicId: 'epic' } } },
      { command: 'load_product_decision_history', args: { input: { epicId: 'epic', decisionId: 'decision' } } },
      expect.objectContaining({ command: 'accept_product_decision_version' }),
    ]);
  });
});
