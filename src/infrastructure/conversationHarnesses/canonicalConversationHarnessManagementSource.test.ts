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

function harnessDetails(replacements: HarnessDetails['replacements'] = []): HarnessDetails {
  return {
    harness: {
      harnessId,
      name: 'Epic Plan Builder',
      createdAt,
      updatedAt: createdAt,
    },
    draft: null,
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
    publishSessionOverride: vi.fn(async () => {
      throw new Error('not used');
    }),
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
    expect(harnesses.orderReplacement).toHaveBeenCalledTimes(1);
    expect(harnesses.orderReplacement).toHaveBeenCalledWith({
      source: reference(1),
      target: reference(2),
    });
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
