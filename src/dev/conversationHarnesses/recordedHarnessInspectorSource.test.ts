import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from './recordedHarnessInspectorSource';

describe('recorded Harness Management source', () => {
  it('resolves one complete recorded revision through the Session-owned relationship', async () => {
    const source = createRecordedHarnessManagementSource();
    const read = await source.load({ sessionId: recordedHarnessInspectorSessionId });

    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;
    expect(read.snapshot).toMatchObject({
      sessionId: recordedHarnessInspectorSessionId,
      harnessKey: 'epic_plan_builder',
      catalogRevision: 4,
      versionControl: {
        support: 'recorded_preview',
        committedRevision: 4,
        activeRevision: 4,
      },
      sessionBinding: {
        state: 'update_available',
        appliedRevision: 3,
        desiredRevision: 4,
      },
    });
    expect(read.snapshot.agentIdentity).toMatchObject({
      harnessRole: 'Epic Plan Builder',
      appliedHarnessRevision: 3,
      assignment: { kind: 'recorded_preview', pool: 'harness_subset' },
    });
    expect(read.snapshot.agentIdentity?.appliedHarnessRevision).toBe(
      read.snapshot.sessionBinding.appliedRevision,
    );
    expect(read.snapshot.workingCopy.configuration).toMatchObject({
      identity: {
        name: 'Epic Plan Builder',
        machineKey: 'epic_plan_builder',
        role: 'Epic Plan Builder',
      },
      promptPrefix: {
        initialDelivery: 'prepend',
        contextCompressionDelivery: 'deferred',
      },
      skills: { discoveryPolicy: 'whitelist' },
      tools: { discoveryPolicy: 'whitelist' },
      runtime: {
        allowInheritedModel: true,
        allowInheritedReasoning: true,
        sandbox: 'read_only',
        approvalPolicy: 'never',
      },
      updatePolicy: { status: 'configured', promptReconstruction: 'deferred' },
    });
    expect(read.snapshot.workingCopy.configuration.skills.items[0]?.policy).toBe('available');
    expect(read.snapshot.workingCopy.configuration.tools.items.map((tool) => tool.name)).toEqual([
      'submit_epic_plan_proposal',
      'request_epic_initiation',
    ]);
  });

  it('persists the complete working copy across views without renaming the existing Session', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const identity = initial.snapshot.agentIdentity;
    const configuration = initial.snapshot.workingCopy.configuration;

    const saved = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'save_working_copy',
        expectedDraftRevision: initial.snapshot.workingCopy.draftRevision,
        configuration: {
          ...configuration,
          promptPrefix: { ...configuration.promptPrefix, content: '# Revised prefix' },
          identity: {
            ...configuration.identity,
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
    expect(reopened.snapshot.workingCopy).toMatchObject({ state: 'uncommitted' });
    expect(reopened.snapshot.workingCopy.configuration.promptPrefix.content).toBe(
      '# Revised prefix',
    );
    expect(reopened.snapshot.harnessKey).toBe('epic_plan_builder');
    expect(reopened.snapshot.workingCopy.configuration.identity.machineKey).toBe('replacement_key');
    expect(reopened.snapshot.agentIdentity).toEqual(identity);
  });

  it('records commit, local activation, and per-session delivery choices as distinct commands', async () => {
    const source = createRecordedHarnessManagementSource();
    const initial = await source.load({ sessionId: recordedHarnessInspectorSessionId });
    expect(initial.kind).toBe('available');
    if (initial.kind !== 'available' || !source.dispatch) return;
    const configuration = initial.snapshot.workingCopy.configuration;
    const saved = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'save_working_copy',
        expectedDraftRevision: initial.snapshot.workingCopy.draftRevision,
        configuration: {
          ...configuration,
          identity: { ...configuration.identity, name: 'Epic Plan Builder Plus' },
        },
      },
    });
    expect(saved.kind).toBe('available');
    if (saved.kind !== 'available') return;
    const committed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'commit',
        expectedDraftRevision: saved.snapshot.workingCopy.draftRevision,
      },
    });
    expect(committed.kind).toBe('available');
    if (committed.kind !== 'available') return;
    expect(committed.snapshot).toMatchObject({
      workingCopy: { state: 'committed_not_active' },
      versionControl: { committedRevision: 5, activeRevision: 4 },
    });
    const pushed = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: { kind: 'push', expectedCommittedRevision: 5 },
    });
    expect(pushed.kind).toBe('available');
    if (pushed.kind !== 'available') return;
    expect(pushed.snapshot).toMatchObject({
      workingCopy: { state: 'clean' },
      versionControl: { committedRevision: 5, activeRevision: 5 },
      sessionBinding: { state: 'update_available', appliedRevision: 3, desiredRevision: 5 },
    });
    const queued = await source.dispatch({
      sessionId: recordedHarnessInspectorSessionId,
      command: {
        kind: 'request_session_update',
        expectedActiveRevision: 5,
        scope: 'current_session',
        strategy: 'next_prompt',
      },
    });
    expect(queued).toMatchObject({
      kind: 'available',
      snapshot: {
        sessionBinding: { state: 'queued', desiredRevision: 5, updateStrategy: 'next_prompt' },
      },
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
