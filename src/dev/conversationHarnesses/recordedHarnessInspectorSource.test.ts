import {
  recordedHarnessInspectorSessionId,
  recordedHarnessInspectorSource,
} from './recordedHarnessInspectorSource';

describe('recordedHarnessInspectorSource', () => {
  it('adapts the checked-in v2 profile without claiming live product provenance', async () => {
    const read = await recordedHarnessInspectorSource.load({
      sessionId: recordedHarnessInspectorSessionId,
    });

    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;
    expect(read.snapshot.profile).toMatchObject({
      key: 'epic_plan_builder',
      version: 4,
      catalogSchemaVersion: 2,
    });
    expect(read.snapshot.provenance.kind).toBe('recorded_adapter');
    expect(read.snapshot.validation.status).toBe('unverified');
    expect(read.snapshot.promptContext.state).toMatchObject({
      scope: 'profile_configuration',
      editability: 'read_only',
    });
    expect(read.snapshot.promptContext.delivery).toMatchObject({
      policy: 'first_query',
      status: 'not_evidenced',
    });
    expect(read.snapshot.mcp.tools).toEqual([
      'submit_epic_plan_proposal',
      'request_epic_initiation',
    ]);
    expect(read.snapshot.runtime).toMatchObject({
      model: null,
      reasoningEffort: null,
      sandbox: 'read_only',
      approvalPolicy: 'never',
    });
    expect(read.snapshot.apply.status).toBe('unsupported');
  });

  it('does not invent a harness for an unbound session', async () => {
    await expect(recordedHarnessInspectorSource.load({ sessionId: 'not-bound' })).resolves.toEqual({
      kind: 'unbound',
      reason: 'This recorded Agent Session has no product harness configuration.',
    });
  });
});
