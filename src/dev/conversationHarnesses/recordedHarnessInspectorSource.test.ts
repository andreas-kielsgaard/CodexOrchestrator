import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from './recordedHarnessInspectorSource';

describe('recorded Harness Management source', () => {
  it('resolves committed revisions, catalogs, and one Session-owned applied revision', async () => {
    const source = createRecordedHarnessManagementSource();
    const read = await source.load({ sessionId: recordedHarnessInspectorSessionId });

    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;
    expect(read.snapshot).toMatchObject({
      sessionId: recordedHarnessInspectorSessionId,
      harnessKey: 'epic_plan_builder',
      workingCopy: null,
      versionControl: {
        support: 'recorded_preview',
        pushedRevision: 4,
      },
      sessionBinding: {
        state: 'behind',
        appliedRevision: 3,
        desiredRevision: null,
        executingPreviousInvocation: false,
      },
    });
    expect(read.snapshot.versionControl.versions.map((version) => version.revision)).toEqual([
      3, 4,
    ]);
    expect(read.snapshot.agentIdentity).toMatchObject({
      harnessRole: 'Epic Plan Builder',
      appliedHarnessRevision: 3,
      assignment: { kind: 'recorded_preview', pool: 'harness_subset' },
    });
    expect(read.snapshot.agentIdentity?.appliedHarnessRevision).toBe(
      read.snapshot.sessionBinding.appliedRevision,
    );
    const current = read.snapshot.versionControl.versions.at(-1)?.configuration;
    expect(current).toMatchObject({
      identity: {
        name: 'Epic Plan Builder',
        machineKey: 'epic_plan_builder',
      },
      promptPrefix: {
        initialDelivery: 'prepend',
        contextCompressionDelivery: 'deferred',
      },
      skills: { availableDiscoveryPolicy: 'whitelist' },
      tools: { availableDiscoveryPolicy: 'whitelist' },
      runtime: {
        defaultModel: null,
        defaultReasoning: null,
        sandbox: 'read_only',
        approvalPolicy: 'never',
      },
      updatePolicy: {
        status: 'configured',
        delivery: 'next_prompt',
        promptReconstruction: 'deferred',
      },
    });
    expect(read.snapshot.catalogs.skills.source).toBe('checked_in_product_catalog');
    expect(read.snapshot.catalogs.skills.items.length).toBeGreaterThan(20);
    expect(read.snapshot.catalogs.skills.items.map((skill) => skill.name)).toContain(
      'epic-plan-builder',
    );
    expect(read.snapshot.catalogs.models).toMatchObject({ source: 'recorded_catalog' });
    expect(current?.tools.items.map((tool) => tool.name)).toEqual([
      'submit_epic_plan_proposal',
      'request_epic_initiation',
    ]);
    expect(current?.hooks).toEqual([
      {
        name: 'proposal persisted or user ends discussion',
        status: 'proposed',
        detail: 'Proposed application hook reference: proposal_persisted_or_user_ends_discussion.',
      },
    ]);
    expect(current?.hooks.map((hook) => hook.status)).not.toContain('exposed');
  });

  it('persists a complete draft across views without renaming the existing Session', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const identity = initial.snapshot.agentIdentity;
    const base = initial.snapshot.versionControl.versions.find((version) => version.revision === 3);
    expect(base).toBeDefined();
    if (!base) return;
    const started = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'start_edit', baseRevision: 3 },
    });
    expect(started.kind).toBe('available');
    if (started.kind !== 'available' || !started.snapshot.workingCopy) return;

    const saved = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'save_working_copy',
        configuration: {
          ...base.configuration,
          promptPrefix: { ...base.configuration.promptPrefix, content: '# Revised prefix' },
          identity: {
            ...base.configuration.identity,
            machineKey: 'replacement_key',
            permittedAgentNames: ['Grace Hopper'],
          },
        },
      },
    });
    const reopened = await source.load({ sessionId: recordedHarnessInspectorSessionId });

    expect(saved.kind).toBe('available');
    expect(reopened.kind).toBe('available');
    if (reopened.kind !== 'available') return;
    expect(reopened.snapshot.workingCopy).toMatchObject({ dirty: true });
    expect(reopened.snapshot.workingCopy?.configuration.promptPrefix.content).toBe(
      '# Revised prefix',
    );
    expect(reopened.snapshot.harnessKey).toBe('epic_plan_builder');
    expect(reopened.snapshot.workingCopy?.configuration.identity.machineKey).toBe(
      'replacement_key',
    );
    expect(reopened.snapshot.agentIdentity).toEqual(identity);
  });

  it('keeps commit, push, and next-prompt Session changes as distinct recorded commands', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const base = initial.snapshot.versionControl.versions.at(-1);
    expect(base).toBeDefined();
    if (!base) return;
    const started = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'start_edit', baseRevision: base.revision },
    });
    expect(started.kind).toBe('available');
    if (started.kind !== 'available' || !started.snapshot.workingCopy) return;
    const saved = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'save_working_copy',
        configuration: {
          ...base.configuration,
          identity: { ...base.configuration.identity, name: 'Epic Plan Builder Plus' },
        },
      },
    });
    expect(saved.kind).toBe('available');
    if (saved.kind !== 'available' || !saved.snapshot.workingCopy) return;
    const committed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'commit',
        expectedDraftRevision: saved.snapshot.workingCopy.draftRevision,
      },
    });
    expect(committed).toMatchObject({
      kind: 'available',
      snapshot: {
        workingCopy: null,
        versionControl: { pushedRevision: 4 },
        sessionBinding: { state: 'behind', desiredRevision: null },
      },
    });
    if (committed.kind !== 'available') return;
    expect(committed.snapshot.versionControl.versions.at(-1)?.revision).toBe(5);

    const pushed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'push', revision: 5 },
    });
    expect(pushed).toMatchObject({
      kind: 'available',
      snapshot: {
        versionControl: { pushedRevision: 5 },
        sessionBinding: {
          state: 'queued',
          appliedRevision: 3,
          desiredRevision: 5,
          executingPreviousInvocation: false,
        },
      },
    });

    const queued = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'queue_version',
        revision: 4,
        scope: 'all_relevant_sessions',
      },
    });
    expect(queued).toMatchObject({
      kind: 'available',
      snapshot: {
        sessionBinding: { state: 'queued', appliedRevision: 3, desiredRevision: 4 },
      },
    });
    if (queued.kind === 'available')
      expect(queued.snapshot.sessionBinding.reason).toMatch(/next prompt/i);
  });

  it('rejects a default reasoning value outside the allowed model range', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const base = initial.snapshot.versionControl.versions.at(-1);
    if (!base) return;
    const started = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'start_edit', baseRevision: base.revision },
    });
    if (started.kind !== 'available' || !started.snapshot.workingCopy) return;
    const configuration = started.snapshot.workingCopy.configuration;
    const rejected = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'save_working_copy',
        configuration: {
          ...configuration,
          runtime: {
            ...configuration.runtime,
            models: configuration.runtime.models.map((model) =>
              model.modelId === 'gpt-5.6-terra'
                ? { ...model, minReasoning: 'high', maxReasoning: 'xhigh' }
                : model,
            ),
            defaultModel: 'gpt-5.6-terra',
            defaultReasoning: 'low',
          },
        },
      },
    });

    expect(rejected).toEqual({
      kind: 'unavailable',
      reason: 'Default reasoning must fit the default model range.',
    });
  });

  it('does not invent a relationship for an unbound Session', async () => {
    const source = createRecordedHarnessManagementSource();
    await expect(source.load({ sessionId: 'not-bound' })).resolves.toEqual({
      kind: 'unbound',
      reason: 'This Agent Session has no harness relationship.',
    });
  });
});
