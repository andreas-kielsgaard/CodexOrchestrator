import type {
  AgentSessionClient,
  AgentSessionDto,
  AgentSessionSummaryDto,
} from '../../application/agentSessions';
import type { HarnessEffectiveConfiguration } from '../../application/conversationHarnesses';
import {
  createHarnessId,
  createHarnessVersionNumber,
  createHarnessVersionRef,
  InMemorySessionHarnessOverrideDraftCache,
  type HarnessDetails,
  type HarnessManagementClient,
  type HarnessVersionRef,
  type PublishedHarnessVersion,
} from '../../application/harnesses';
import { exampleHarnessConfiguration } from '../../application/harnesses/testFixtures';
import { createCanonicalConversationHarnessManagementSource } from './canonicalConversationHarnessManagementSource';

const harnessId = createHarnessId('harness-epic-plan-builder');
const otherHarnessId = createHarnessId('harness-reviewer');
const reference = (version: number) =>
  createHarnessVersionRef(harnessId, createHarnessVersionNumber(version));
const otherReference = (version: number) =>
  createHarnessVersionRef(otherHarnessId, createHarnessVersionNumber(version));
const createdAt = '2026-08-27T08:00:00Z';

function publishedVersion(version: number): PublishedHarnessVersion {
  return {
    reference: reference(version),
    scope: { kind: 'reusable' },
    configuration: exampleHarnessConfiguration(),
    createdAt,
  };
}

function harnessDetails(
  replacements: HarnessDetails['replacements'] = [],
  draft: HarnessDetails['draft'] = null,
): HarnessDetails {
  return {
    harness: {
      harnessId,
      name: 'Epic Plan Builder',
      createdAt,
      updatedAt: createdAt,
    },
    draft,
    versions: [publishedVersion(1), publishedVersion(2)],
    replacements,
  };
}

function session(id: string, harnessVersion: HarnessVersionRef | null): AgentSessionDto {
  return {
    id,
    title: id,
    availability: 'available',
    runtimeBinding: { externalContextId: null, runtimeVersion: null },
    workingDirectory: null,
    requestedOptions: { model: null, sandbox: null },
    harnessVersion,
    assignedIdentity: null,
    createdAt,
    updatedAt: createdAt,
  };
}

function summary(value: AgentSessionDto): AgentSessionSummaryDto {
  return {
    id: value.id,
    title: value.title,
    availability: value.availability,
    hasActiveInvocation: false,
    latestInvocationStatus: null,
    createdAt: value.createdAt,
    updatedAt: value.updatedAt,
  };
}

function createSessionClient(values: readonly AgentSessionDto[]): AgentSessionClient {
  const sessions = new Map(values.map((value) => [value.id, value]));
  return {
    createSession: vi.fn(async () => {
      throw new Error('not used');
    }),
    updateHarness: vi.fn(async ({ sessionId, harnessVersion }) => {
      const current = sessions.get(sessionId);
      if (!current) throw new Error(`Unknown Session: ${sessionId}`);
      const updated = { ...current, harnessVersion };
      sessions.set(sessionId, updated);
      return updated;
    }),
    updateIdentity: vi.fn(async ({ sessionId, assignedIdentity }) => {
      const current = sessions.get(sessionId);
      if (!current) throw new Error(`Unknown Session: ${sessionId}`);
      const updated = { ...current, assignedIdentity };
      sessions.set(sessionId, updated);
      return updated;
    }),
    updateModelOverride: vi.fn(async ({ sessionId, model }) => {
      const current = sessions.get(sessionId);
      if (!current) throw new Error(`Unknown Session: ${sessionId}`);
      const updated = {
        ...current,
        requestedOptions: { ...current.requestedOptions, model },
      };
      sessions.set(sessionId, updated);
      return updated;
    }),
    listSessions: vi.fn(async () => [...sessions.values()].map(summary)),
    loadSession: vi.fn(async ({ sessionId }) => {
      const value = sessions.get(sessionId);
      if (!value) throw new Error(`Unknown Session: ${sessionId}`);
      return { session: value, invocations: [] };
    }),
    reloadSession: vi.fn(async ({ sessionId }) => {
      const value = sessions.get(sessionId);
      if (!value) throw new Error(`Unknown Session: ${sessionId}`);
      return { session: value, invocations: [] };
    }),
    subscribeUpdates: vi.fn(async () => () => undefined),
    sendMessage: vi.fn(async () => {
      throw new Error('not used');
    }),
    cancelInvocation: vi.fn(async () => {
      throw new Error('not used');
    }),
    disconnectUpdates: vi.fn(async () => undefined),
  };
}

function createHarnessClient(
  details: HarnessDetails,
  resolved = publishedVersion(2),
): HarnessManagementClient {
  return {
    list: vi.fn(async () => [details.harness]),
    load: vi.fn(async () => details),
    create: vi.fn(async () => {
      throw new Error('not used');
    }),
    rename: vi.fn(async () => {
      throw new Error('not used');
    }),
    saveDraft: vi.fn(async ({ harnessId: id, basedOn, configuration }) => ({
      harnessId: id,
      basedOn,
      configuration,
      savedAt: createdAt,
    })),
    publishDraft: vi.fn(async () => resolved),
    publishSessionOverride: vi.fn(async ({ sessionId, configuration }) => ({
      reference: reference(3),
      scope: { kind: 'session_specific' as const, sessionId },
      configuration,
      createdAt,
    })),
    orderReplacement: vi.fn(async ({ source, target }) => ({ source, target })),
    resolveVersion: vi.fn(async ({ requested }) => ({
      requested,
      version: resolved,
      replacementPath:
        requested.version === resolved.reference.version
          ? [requested]
          : [requested, resolved.reference],
    })),
  };
}

async function availableConfiguration(
  source: ReturnType<typeof createCanonicalConversationHarnessManagementSource>,
): Promise<HarnessEffectiveConfiguration> {
  const read = await source.load({ sessionId: 'session-current' });
  if (read.kind !== 'available') throw new Error(`Expected available read, received ${read.kind}`);
  return read.snapshot.versionControl.versions[1]!.configuration;
}

describe('canonical Conversation Harness Management source', () => {
  it('loads the exact Harness reference owned by the Session and exposes replacement-behind state', async () => {
    const requested = reference(1);
    const resolved = publishedVersion(2);
    const sessions = createSessionClient([session('session-current', requested)]);
    const harnesses = createHarnessClient(
      harnessDetails([{ source: requested, target: resolved.reference }]),
      resolved,
    );
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses);

    const read = await source.load({ sessionId: 'session-current' });

    expect(sessions.loadSession).toHaveBeenCalledWith({ sessionId: 'session-current' });
    expect(harnesses.load).toHaveBeenCalledWith({ harnessId });
    expect(harnesses.resolveVersion).toHaveBeenCalledWith({ requested });
    expect(read).toMatchObject({
      kind: 'available',
      snapshot: {
        sessionId: 'session-current',
        harnessKey: harnessId,
        sessionBinding: {
          state: 'behind',
          appliedRevision: 1,
          desiredRevision: 2,
        },
      },
    });
  });

  it('dispatches draft save, publication, and replacement push through the Harness engine', async () => {
    const sessions = createSessionClient([session('session-current', reference(1))]);
    const harnesses = createHarnessClient(harnessDetails());
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses);
    const configuration = await availableConfiguration(source);
    const edited = {
      ...configuration,
      promptPrefix: { ...configuration.promptPrefix, content: 'Revised product context.' },
    };

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'save_working_copy', configuration: edited },
    });
    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'commit', expectedDraftRevision: 1 },
    });
    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'push', revision: 2 },
    });

    expect(harnesses.saveDraft).toHaveBeenCalledWith(
      expect.objectContaining({
        harnessId,
        basedOn: reference(2),
        configuration: expect.objectContaining({
          promptPrefix: expect.objectContaining({ content: 'Revised product context.' }),
        }),
      }),
    );
    expect(harnesses.publishDraft).toHaveBeenCalledWith({ harnessId });
    expect(harnesses.publishSessionOverride).not.toHaveBeenCalled();
    expect(harnesses.orderReplacement).toHaveBeenCalledTimes(1);
    expect(harnesses.orderReplacement).toHaveBeenCalledWith({
      source: reference(1),
      target: reference(2),
    });
  });

  it('keeps Session customization in memory and publishes it from the exact base reference', async () => {
    const sessions = createSessionClient([session('session-current', reference(2))]);
    const harnesses = createHarnessClient(harnessDetails());
    const drafts = new InMemorySessionHarnessOverrideDraftCache();
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses, drafts);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'start_session_edit', baseRevision: 1 },
    });
    const started = drafts.get('session-current');
    expect(started?.baseHarnessRef).toEqual(reference(1));
    expect(harnesses.saveDraft).not.toHaveBeenCalled();

    const reopened = await source.load({ sessionId: 'session-current' });
    expect(reopened).toMatchObject({
      kind: 'available',
      snapshot: { sessionWorkingCopy: { baseRevision: 1, dirty: true } },
    });
    if (reopened.kind !== 'available' || !reopened.snapshot.sessionWorkingCopy)
      throw new Error('Expected an in-memory Session working copy.');
    const edited = {
      ...reopened.snapshot.sessionWorkingCopy.configuration,
      promptPrefix: {
        ...reopened.snapshot.sessionWorkingCopy.configuration.promptPrefix,
        content: 'Only this Session receives this context.',
      },
    };

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'save_session_working_copy', configuration: edited },
    });

    expect(drafts.get('session-current')).toMatchObject({
      baseHarnessRef: reference(1),
      configuration: { promptPrefix: { content: 'Only this Session receives this context.' } },
    });
    expect(harnesses.saveDraft).not.toHaveBeenCalled();

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'publish_session_override', expectedBaseRevision: 1 },
    });

    expect(harnesses.publishSessionOverride).toHaveBeenCalledWith({
      harnessId,
      sessionId: 'session-current',
      baseHarnessRef: reference(1),
      configuration: expect.objectContaining({
        promptPrefix: expect.objectContaining({
          content: 'Only this Session receives this context.',
        }),
      }),
    });
    expect(sessions.updateHarness).toHaveBeenCalledWith({
      sessionId: 'session-current',
      harnessVersion: reference(3),
    });
    expect(drafts.get('session-current')).toBeNull();
    expect(harnesses.saveDraft).not.toHaveBeenCalled();
  });

  it('discards only the in-memory Session customization', async () => {
    const sessions = createSessionClient([session('session-current', reference(2))]);
    const harnesses = createHarnessClient(harnessDetails());
    const drafts = new InMemorySessionHarnessOverrideDraftCache();
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses, drafts);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'start_session_edit', baseRevision: 2 },
    });
    expect(drafts.get('session-current')).not.toBeNull();

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'discard_session_working_copy' },
    });

    expect(drafts.get('session-current')).toBeNull();
    expect(harnesses.publishSessionOverride).not.toHaveBeenCalled();
    expect(harnesses.saveDraft).not.toHaveBeenCalled();
    expect(sessions.updateHarness).not.toHaveBeenCalled();
  });

  it('exposes independent reusable Harness and Session drafts at the same time', async () => {
    const persistentDraft = {
      harnessId,
      basedOn: reference(2),
      configuration: exampleHarnessConfiguration(),
      savedAt: createdAt,
    };
    const sessions = createSessionClient([session('session-current', reference(2))]);
    const harnesses = createHarnessClient(harnessDetails([], persistentDraft));
    const drafts = new InMemorySessionHarnessOverrideDraftCache();
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses, drafts);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'start_session_edit', baseRevision: 1 },
    });
    const read = await source.load({ sessionId: 'session-current' });

    expect(read).toMatchObject({
      kind: 'available',
      snapshot: {
        workingCopy: { baseRevision: 2 },
        sessionWorkingCopy: { baseRevision: 1 },
      },
    });
    expect(harnesses.saveDraft).not.toHaveBeenCalled();
  });

  it('updates presentation identity on the Agent Session rather than on the Harness', async () => {
    const sessions = createSessionClient([session('session-current', reference(2))]);
    const harnesses = createHarnessClient(harnessDetails());
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: {
        kind: 'update_session_identity',
        name: 'Avery',
        visualIdentity: { token: 'identity-avery', accent: '#4f46e5', shape: 'hexagon' },
      },
    });

    expect(sessions.updateIdentity).toHaveBeenCalledWith({
      sessionId: 'session-current',
      assignedIdentity: {
        originIdentityId: 'identity-avery',
        displayName: 'Avery',
        color: '#4f46e5',
        shape: 'hexagon',
      },
    });
    expect(harnesses.saveDraft).not.toHaveBeenCalled();
  });

  it('sets and clears the model override on the Agent Session', async () => {
    const sessions = createSessionClient([session('session-current', reference(2))]);
    const harnesses = createHarnessClient(harnessDetails());
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: {
        kind: 'set_session_model_override',
        override: { model: 'application-model', reasoning: null },
      },
    });
    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'set_session_model_override', override: null },
    });

    expect(sessions.updateModelOverride).toHaveBeenNthCalledWith(1, {
      sessionId: 'session-current',
      model: 'application-model',
    });
    expect(sessions.updateModelOverride).toHaveBeenNthCalledWith(2, {
      sessionId: 'session-current',
      model: null,
    });
    expect(harnesses.saveDraft).not.toHaveBeenCalled();
  });

  it('updates every available Session attached to the same Harness and leaves others unchanged', async () => {
    const sessions = createSessionClient([
      session('session-current', reference(1)),
      session('session-relevant', reference(1)),
      session('session-other-harness', otherReference(1)),
      session('session-unbound', null),
    ]);
    const harnesses = createHarnessClient(harnessDetails());
    const source = createCanonicalConversationHarnessManagementSource(sessions, harnesses);

    await source.dispatch?.({
      sessionId: 'session-current',
      command: { kind: 'queue_version', revision: 2, scope: 'all_relevant_sessions' },
    });

    expect(sessions.updateHarness).toHaveBeenCalledTimes(2);
    expect(sessions.updateHarness).toHaveBeenCalledWith({
      sessionId: 'session-current',
      harnessVersion: reference(2),
    });
    expect(sessions.updateHarness).toHaveBeenCalledWith({
      sessionId: 'session-relevant',
      harnessVersion: reference(2),
    });
    expect(sessions.updateHarness).not.toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-other-harness' }),
    );
    expect(sessions.updateHarness).not.toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: 'session-unbound' }),
    );
  });
});
