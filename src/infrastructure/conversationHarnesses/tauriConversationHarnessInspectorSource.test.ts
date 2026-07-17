import { createTauriConversationHarnessInspectorSource } from './tauriConversationHarnessInspectorSource';

const profile = {
  key: 'epic_plan_builder',
  version: 4,
  context: 'Product-owned first-query context.',
  skillGuidance: [
    {
      canonicalName: 'epic-plan-builder',
      canonicalPath: '.agents/skills/epic-plan-builder/SKILL.md',
      purpose: 'Build one Epic plan.',
      useWhen: 'the user requests a plan.',
    },
  ],
  runtime: {
    model: null,
    reasoningEffort: null,
    sandbox: 'read_only',
    approvalPolicy: 'never',
  },
  mcp: {
    required: true,
    enabledTools: ['submit_epic_plan_proposal', 'request_epic_initiation'],
  },
  lifecycle: {
    contextDelivery: 'first_query',
    completionCriteria: ['proposal_saved'],
  },
} as const;

describe('Tauri Conversation Harness inspector source', () => {
  it('adapts the bound product query and preserves durable delivery states', async () => {
    const invoke = vi.fn().mockResolvedValue({
      kind: 'bound',
      sessionId: 'session-1',
      catalogSchemaVersion: 2,
      profile,
      delivery: {
        status: 'delivered',
        invocationId: 'invocation-1',
      },
    });
    const source = createTauriConversationHarnessInspectorSource(invoke);

    const read = await source.load({ sessionId: 'session-1' });

    expect(invoke).toHaveBeenCalledWith('load_managed_plan_builder_harness_inspection', {
      input: { sessionId: 'session-1' },
    });
    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;
    expect(read.snapshot).toMatchObject({
      sessionId: 'session-1',
      profile: {
        key: 'epic_plan_builder',
        version: 4,
        catalogSchemaVersion: 2,
      },
      provenance: { kind: 'product_query' },
      validation: { status: 'unverified' },
      promptContext: {
        delivery: { status: 'delivered' },
      },
    });
    expect(read.snapshot.promptContext.delivery.detail).toContain('Durable launch acceptance');

    invoke.mockResolvedValueOnce({
      kind: 'bound',
      sessionId: 'session-1',
      catalogSchemaVersion: 2,
      profile,
      delivery: {
        status: 'not_delivered',
        reason: 'launch_rejected',
      },
    });
    const notDelivered = await source.load({ sessionId: 'session-1' });
    expect(notDelivered.kind).toBe('available');
    if (notDelivered.kind === 'available')
      expect(notDelivered.snapshot.promptContext.delivery.status).toBe('not_delivered');

    invoke.mockResolvedValueOnce({
      kind: 'bound',
      sessionId: 'session-1',
      catalogSchemaVersion: 2,
      profile,
      delivery: {
        status: 'not_evidenced',
        invocationId: 'invocation-1',
        reason: 'launch_acceptance_missing',
      },
    });
    const notEvidenced = await source.load({ sessionId: 'session-1' });
    expect(notEvidenced.kind).toBe('available');
    if (notEvidenced.kind === 'available') {
      expect(notEvidenced.snapshot.promptContext.delivery.status).toBe('not_evidenced');
      expect(notEvidenced.snapshot.validation.checks.at(-1)?.status).toBe('unverified');
    }
  });

  it('keeps unbound, invalid-catalog, unavailable, and invalid transport states distinct', async () => {
    const invoke = vi.fn();
    const source = createTauriConversationHarnessInspectorSource(invoke);

    invoke.mockResolvedValueOnce({ kind: 'unbound', sessionId: 'session-1' });
    await expect(source.load({ sessionId: 'session-1' })).resolves.toMatchObject({
      kind: 'unbound',
    });

    invoke.mockResolvedValueOnce({ kind: 'invalid_catalog', sessionId: 'session-1' });
    await expect(source.load({ sessionId: 'session-1' })).resolves.toMatchObject({
      kind: 'invalid_catalog',
    });

    invoke.mockRejectedValueOnce(new Error('transport unavailable'));
    await expect(source.load({ sessionId: 'session-1' })).resolves.toMatchObject({
      kind: 'unavailable',
    });

    invoke.mockResolvedValueOnce({ kind: 'bound', sessionId: 'session-1' });
    await expect(source.load({ sessionId: 'session-1' })).resolves.toMatchObject({
      kind: 'unavailable',
    });
  });
});
