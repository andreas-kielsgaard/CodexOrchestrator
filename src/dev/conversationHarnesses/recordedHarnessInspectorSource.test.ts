import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorPeerSessionId,
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
    expect(read.snapshot.versionControl.versions.map((version) => version.status)).toEqual([
      'pushed',
      'pushed',
    ]);
    expect(
      read.snapshot.versionControl.versions.map(
        (version) => version.configuration.runtime.modelPolicyMode,
      ),
    ).toEqual(['delegated_shared', 'revision_owned']);
    expect(read.snapshot.modelChoices.delegatedPolicies.map(({ revision }) => revision)).toEqual([
      3,
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
    expect(read.snapshot.catalogs.agentNames).toMatchObject({
      source: 'product_default_pool',
    });
    expect(read.snapshot.catalogs.agentNames.items).toHaveLength(100);
    expect(read.snapshot.catalogs.skills.items.map((skill) => skill.name)).toContain(
      'epic-plan-builder',
    );
    expect(
      read.snapshot.catalogs.skills.items.find((skill) => skill.name === 'epic-plan-builder')?.text,
    ).toContain('# Epic Plan Builder');
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
    expect(committed.snapshot.versionControl.versions.at(-1)).toMatchObject({
      revision: 5,
      status: 'committed',
    });

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
    if (pushed.kind === 'available')
      expect(
        pushed.snapshot.versionControl.versions.find((version) => version.revision === 5)?.status,
      ).toBe('pushed');

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

  it('keeps delegated shared policies, Session overrides, and user preference separately owned', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    const peerInitial = await source.load({ sessionId: recordedHarnessInspectorPeerSessionId });
    expect(initial.kind).toBe('available');
    expect(peerInitial.kind).toBe('available');
    if (initial.kind !== 'available' || peerInitial.kind !== 'available' || !source.dispatch)
      return;
    const version = initial.snapshot.versionControl.versions.find(({ revision }) => revision === 3);
    expect(version).toBeDefined();
    if (!version) return;
    const sharedPolicy = {
      models: version.configuration.runtime.models,
      defaultModel: 'gpt-5.6-sol',
      defaultReasoning: 'high' as const,
    };

    const proposed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'save_delegated_model_policy', revision: 3, policy: sharedPolicy },
    });
    expect(proposed.kind).toBe('available');
    if (proposed.kind !== 'available') return;
    expect(
      proposed.snapshot.versionControl.versions.find(({ revision }) => revision === 3)
        ?.configuration.runtime.defaultModel,
    ).toBeNull();
    expect(proposed.snapshot.modelChoices.delegatedPolicies).toContainEqual(
      expect.objectContaining({ revision: 3, policy: sharedPolicy, dirty: true }),
    );
    expect(proposed.snapshot.modelChoices.userPreference).toMatchObject({
      lastUsedModel: 'gpt-5.6-terra',
      lastUsedReasoning: 'high',
      support: 'recorded_preference_register',
    });
    expect(proposed.snapshot.modelChoices.resolvedForCurrentSession).toMatchObject({
      model: 'gpt-5.6-sol',
      reasoning: 'high',
      source: 'delegated_shared_policy',
    });
    const peerAfterSharedChange = await source.load({
      sessionId: recordedHarnessInspectorPeerSessionId,
    });
    expect(peerAfterSharedChange).toMatchObject({
      kind: 'available',
      snapshot: {
        modelChoices: {
          delegatedPolicies: [expect.objectContaining({ revision: 3, policy: sharedPolicy })],
          sessionOverride: null,
          resolvedForCurrentSession: {
            model: 'gpt-5.6-sol',
            reasoning: 'high',
            source: 'delegated_shared_policy',
          },
        },
      },
    });

    const outsideConstraints = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'set_session_model_override',
        override: { model: 'gpt-5.6-sol', reasoning: 'low' },
      },
    });
    expect(outsideConstraints).toEqual({
      kind: 'unavailable',
      reason: 'The Session choice must fit its applied Harness policy.',
    });

    const overridden = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'set_session_model_override',
        override: { model: 'gpt-5.6-terra', reasoning: 'xhigh' },
      },
    });
    expect(overridden.kind).toBe('available');
    if (overridden.kind !== 'available') return;
    expect(overridden.snapshot.modelChoices.sessionOverride).toEqual({
      model: 'gpt-5.6-terra',
      reasoning: 'xhigh',
    });
    expect(overridden.snapshot.modelChoices.resolvedForCurrentSession).toMatchObject({
      model: 'gpt-5.6-terra',
      reasoning: 'xhigh',
      source: 'session_override',
    });
    expect(
      overridden.snapshot.modelChoices.delegatedPolicies.find(({ revision }) => revision === 3)
        ?.policy,
    ).toEqual(sharedPolicy);

    const restored = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'set_session_model_override', override: null },
    });
    expect(restored.kind).toBe('available');
    if (restored.kind !== 'available') return;
    expect(restored.snapshot.modelChoices.sessionOverride).toBeNull();
    expect(restored.snapshot.modelChoices.resolvedForCurrentSession).toMatchObject({
      model: 'gpt-5.6-sol',
      reasoning: 'high',
      source: 'delegated_shared_policy',
    });
  });

  it('records a Session identity change without mutating the Harness name pool', async () => {
    const changes: string[] = [];
    const source = createRecordedHarnessManagementSource({
      onSessionIdentityChange(identity) {
        changes.push(`${identity.name}:${identity.visualIdentity.token}`);
      },
    });
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const permittedNames =
      initial.snapshot.versionControl.versions[0].configuration.identity.permittedAgentNames;
    const runner = initial.snapshot.catalogs.agentVisualIdentities.items.find(
      ({ identity }) => identity.token === 'runner_route',
    );
    expect(runner).toBeDefined();
    if (!runner) return;

    const changed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'update_session_identity',
        name: 'Mildred Plot Twist',
        visualIdentity: runner.identity,
      },
    });
    expect(changed.kind).toBe('available');
    if (changed.kind !== 'available') return;
    expect(changed.snapshot.agentIdentity).toMatchObject({
      name: 'Mildred Plot Twist',
      visualIdentity: runner.identity,
      assignment: { kind: 'recorded_preview' },
    });
    expect(
      changed.snapshot.versionControl.versions[0].configuration.identity.permittedAgentNames,
    ).toEqual(permittedNames);
    expect(changes).toEqual(['Mildred Plot Twist:runner_route']);
  });

  it('does not invent a relationship for an unbound Session', async () => {
    const source = createRecordedHarnessManagementSource();
    await expect(source.load({ sessionId: 'not-bound' })).resolves.toEqual({
      kind: 'unbound',
      reason: 'This Agent Session has no harness relationship.',
    });
  });
});
