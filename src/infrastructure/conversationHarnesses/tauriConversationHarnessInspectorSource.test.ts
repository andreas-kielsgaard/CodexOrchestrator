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

describe('Tauri Conversation Harness Management source', () => {
  it('adapts the product query without inventing version history, identity, or update support', async () => {
    const invoke = vi.fn().mockResolvedValue({
      kind: 'bound',
      sessionId: 'session-1',
      catalogSchemaVersion: 2,
      profile,
      initialPromptLaunchEvidence: {
        status: 'launch_accepted',
        invocationId: 'invocation-1',
      },
    });
    const source = createTauriConversationHarnessInspectorSource(invoke);

    const read = await source.load({ sessionId: 'session-1' });

    expect(invoke).toHaveBeenCalledWith('load_managed_plan_builder_harness_inspection', {
      input: { sessionId: 'session-1' },
    });
    expect(source.dispatch).toBeUndefined();
    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;
    expect(read.snapshot).toMatchObject({
      sessionId: 'session-1',
      harnessKey: 'epic_plan_builder',
      agentIdentity: null,
      versionControl: {
        support: 'not_connected',
        pushedRevision: null,
      },
      sessionBinding: {
        state: 'untracked',
        appliedRevision: null,
        desiredRevision: null,
      },
      workingCopy: null,
      catalogs: {
        agentNames: { source: 'not_connected' },
        skills: { source: 'not_connected' },
        tools: { source: 'not_connected' },
        models: { source: 'not_connected' },
      },
    });
    expect(read.snapshot.versionControl.versions).toHaveLength(1);
    expect(read.snapshot.versionControl.versions[0]).toMatchObject({
      revision: 4,
      status: 'inspected',
      configuration: {
        identity: { name: 'Epic Plan Builder', machineKey: 'epic_plan_builder' },
        hooks: [
          {
            name: 'proposal saved',
            status: 'not_connected',
            detail:
              'Harness catalog reference: proposal_saved. No typed Application hook registry confirms a connection.',
          },
        ],
        updatePolicy: { status: 'not_configured' },
      },
    });
    expect(read.snapshot.catalogs.skills.items[0]?.text).toBeNull();
    expect(read.snapshot.versionControl.versions[0]?.configuration.hooks).not.toContainEqual(
      expect.objectContaining({ status: 'exposed' }),
    );
  });

  it('keeps transport read states distinct while leaving management commands unavailable', async () => {
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

    invoke.mockResolvedValueOnce({
      kind: 'bound',
      sessionId: 'session-1',
      catalogSchemaVersion: 2,
      profile,
      delivery: { status: 'delivered', invocationId: 'invocation-1' },
    });
    await expect(source.load({ sessionId: 'session-1' })).resolves.toMatchObject({
      kind: 'unavailable',
    });
    expect(source.dispatch).toBeUndefined();
  });
});
