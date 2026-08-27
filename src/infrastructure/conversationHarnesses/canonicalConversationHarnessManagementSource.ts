import type { AgentSessionClient, AgentSessionDto } from '../../application/agentSessions';
import type {
  ConversationHarnessManagementCommand,
  ConversationHarnessManagementRead,
  ConversationHarnessManagementSnapshot,
  ConversationHarnessManagementSource,
  HarnessEffectiveConfiguration,
  HarnessReasoningLevel,
} from '../../application/conversationHarnesses';
import type {
  HarnessConfiguration,
  HarnessDetails,
  HarnessManagementClient,
  SessionHarnessOverrideDraft,
  SessionHarnessOverrideDraftCache,
  HarnessVersionRef,
  PublishedHarnessVersion,
} from '../../application/harnesses';
import type { IdentityDefinition, IdentityManagementClient } from '../../application/identities';
import { identityDefinitionFromCatalog } from '../../application/identities';
import {
  createHarnessVersionNumber,
  createHarnessVersionRef,
  InMemorySessionHarnessOverrideDraftCache,
} from '../../application/harnesses';

export function createCanonicalConversationHarnessManagementSource(
  sessions: AgentSessionClient,
  harnesses: HarnessManagementClient,
  identities?: IdentityManagementClient,
  sessionOverrideDrafts: SessionHarnessOverrideDraftCache = new InMemorySessionHarnessOverrideDraftCache(),
): ConversationHarnessManagementSource {
  const load = async ({ sessionId }: { readonly sessionId: string }) =>
    loadSessionHarness(sessions, harnesses, identities, sessionOverrideDrafts, sessionId);

  return {
    load,
    async dispatch({ sessionId, command }) {
      const context = await availableContext(sessions, harnesses, sessionId);
      await dispatchCommand(sessions, harnesses, sessionOverrideDrafts, context, command);
      return load({ sessionId });
    },
  };
}

interface AvailableContext {
  readonly session: AgentSessionDto;
  readonly details: HarnessDetails;
  readonly requested: HarnessVersionRef;
  readonly resolved: PublishedHarnessVersion;
}

async function loadSessionHarness(
  sessions: AgentSessionClient,
  harnesses: HarnessManagementClient,
  identities: IdentityManagementClient | undefined,
  sessionOverrideDrafts: SessionHarnessOverrideDraftCache,
  sessionId: string,
): Promise<ConversationHarnessManagementRead> {
  try {
    const [context, identityCatalog] = await Promise.all([
      availableContext(sessions, harnesses, sessionId),
      loadIdentityCatalog(identities),
    ]);
    return {
      kind: 'available',
      snapshot: snapshot(context, sessionOverrideDrafts.get(sessionId), identityCatalog),
    };
  } catch (error) {
    if (error instanceof UnboundSessionHarness)
      return {
        kind: 'unbound',
        reason: 'This Agent Session does not own a Harness version yet.',
      };
    return {
      kind: 'unavailable',
      reason: error instanceof Error ? error.message : 'Harness Management is unavailable.',
    };
  }
}

interface LoadedIdentityCatalog {
  readonly source: 'application_identity_catalog' | 'not_connected';
  readonly items: readonly IdentityDefinition[];
  readonly reason: string;
}

async function loadIdentityCatalog(
  identities: IdentityManagementClient | undefined,
): Promise<LoadedIdentityCatalog> {
  if (!identities)
    return {
      source: 'not_connected',
      items: [],
      reason: 'The application Identity catalog is unavailable through this source.',
    };
  try {
    return {
      source: 'application_identity_catalog',
      items: (await identities.list()).map(identityDefinitionFromCatalog),
      reason: 'Application-owned reusable identities available to new Agent Sessions.',
    };
  } catch (error) {
    return {
      source: 'not_connected',
      items: [],
      reason:
        error instanceof Error
          ? `The application Identity catalog is unavailable: ${error.message}`
          : 'The application Identity catalog is unavailable.',
    };
  }
}

async function availableContext(
  sessions: AgentSessionClient,
  harnesses: HarnessManagementClient,
  sessionId: string,
): Promise<AvailableContext> {
  const session = (await sessions.loadSession({ sessionId })).session;
  const requested = session.harnessVersion;
  if (!requested) throw new UnboundSessionHarness();
  const [details, resolution] = await Promise.all([
    harnesses.load({ harnessId: requested.harnessId }),
    harnesses.resolveVersion({ requested }),
  ]);
  return { session, details, requested, resolved: resolution.version };
}

async function dispatchCommand(
  sessions: AgentSessionClient,
  harnesses: HarnessManagementClient,
  sessionOverrideDrafts: SessionHarnessOverrideDraftCache,
  context: AvailableContext,
  command: ConversationHarnessManagementCommand,
): Promise<void> {
  const harnessId = context.requested.harnessId;
  switch (command.kind) {
    case 'start_edit':
      if (!context.details.draft)
        await harnesses.saveDraft({
          harnessId,
          basedOn: context.resolved.reference,
          configuration: context.resolved.configuration,
        });
      return;
    case 'save_working_copy': {
      const baseline = context.details.draft?.configuration ?? context.resolved.configuration;
      await harnesses.saveDraft({
        harnessId,
        basedOn: context.details.draft?.basedOn ?? context.resolved.reference,
        configuration: toCanonicalConfiguration(command.configuration, baseline),
      });
      return;
    }
    case 'start_session_edit': {
      const baseHarnessRef = versionRef(context, command.baseRevision);
      const base = configurationForVersion(context, baseHarnessRef);
      sessionOverrideDrafts.save(context.session.id, baseHarnessRef, base);
      return;
    }
    case 'save_session_working_copy': {
      const draft = requireSessionOverrideDraft(sessionOverrideDrafts, context.session.id);
      const baseline = configurationForVersion(context, draft.baseHarnessRef);
      sessionOverrideDrafts.save(
        context.session.id,
        draft.baseHarnessRef,
        toCanonicalConfiguration(command.configuration, baseline),
      );
      return;
    }
    case 'publish_session_override': {
      if (!sessions.updateHarness)
        throw new Error('This Agent Session client cannot update Harness ownership.');
      const draft = requireSessionOverrideDraft(sessionOverrideDrafts, context.session.id);
      if (draft.baseHarnessRef.harnessId !== harnessId)
        throw new Error('The Session now uses a different Harness than this customization.');
      if (draft.baseHarnessRef.version !== command.expectedBaseRevision)
        throw new Error('The Session Harness customization changed while it was being published.');
      const publication = {
        harnessId,
        sessionId: context.session.id,
        baseHarnessRef: draft.baseHarnessRef,
        configuration: draft.configuration,
      };
      const published = await harnesses.publishSessionOverride(publication);
      await sessions.updateHarness({
        sessionId: context.session.id,
        harnessVersion: published.reference,
      });
      sessionOverrideDrafts.discard(context.session.id);
      return;
    }
    case 'discard_session_working_copy':
      sessionOverrideDrafts.discard(context.session.id);
      return;
    case 'commit':
      await harnesses.publishDraft({ harnessId });
      return;
    case 'push': {
      const target = versionRef(context, command.revision);
      for (const version of context.details.versions) {
        if (
          version.scope.kind === 'reusable' &&
          version.reference.version !== target.version &&
          version.reference.version < target.version
        )
          await harnesses.orderReplacement({ source: version.reference, target });
      }
      return;
    }
    case 'queue_version': {
      if (!sessions.updateHarness)
        throw new Error('This Agent Session client cannot update Harness ownership.');
      const target = versionRef(context, command.revision);
      if (command.scope === 'current_session') {
        await sessions.updateHarness({ sessionId: context.session.id, harnessVersion: target });
        return;
      }
      const summaries = await sessions.listSessions({ availability: 'available' });
      for (const summary of summaries) {
        const candidate = (await sessions.loadSession({ sessionId: summary.id })).session;
        if (candidate.harnessVersion?.harnessId === harnessId)
          await sessions.updateHarness({ sessionId: candidate.id, harnessVersion: target });
      }
      return;
    }
    case 'update_session_identity':
      if (!sessions.updateIdentity)
        throw new Error('This Agent Session client cannot update identity ownership.');
      await sessions.updateIdentity({
        sessionId: context.session.id,
        assignedIdentity: {
          originIdentityId: command.visualIdentity.token || null,
          displayName: command.name,
          color: command.visualIdentity.accent,
          shape: command.visualIdentity.shape,
        },
      });
      return;
    case 'set_session_model_override':
      if (!sessions.updateModelOverride)
        throw new Error('This Agent Session client cannot update its model override.');
      await sessions.updateModelOverride({
        sessionId: context.session.id,
        model: command.override?.model ?? null,
      });
      return;
    case 'save_delegated_model_policy':
      throw new Error('Model providers and catalogs are application-global, not Harness-owned.');
  }
}

function snapshot(
  context: AvailableContext,
  sessionOverrideDraft: SessionHarnessOverrideDraft | null,
  identityCatalog: LoadedIdentityCatalog,
): ConversationHarnessManagementSnapshot {
  const { session, details, requested, resolved } = context;
  const pushedVersions = new Set(details.replacements.map(({ target }) => target.version));
  const replacedVersions = new Set(details.replacements.map(({ source }) => source.version));
  const preferredModel = resolved.configuration.runtime.preferredModel;
  const modelItems = preferredModel
    ? [
        {
          id: preferredModel.modelId,
          label: preferredModel.modelId,
          reasoningLevels: preferredModel.reasoning
            ? [preferredModel.reasoning]
            : (['low', 'medium', 'high', 'xhigh'] as const),
        },
      ]
    : [];
  return {
    sessionId: session.id,
    harnessKey: details.harness.harnessId,
    agentIdentity: session.assignedIdentity
      ? {
          name: session.assignedIdentity.displayName,
          harnessRole: details.harness.name,
          visualIdentityToken: session.assignedIdentity.originIdentityId ?? 'session_identity',
          visualIdentityAccent: session.assignedIdentity.color,
          visualIdentityShape: session.assignedIdentity.shape,
        }
      : null,
    catalogs: {
      identities: identityCatalog,
      agentNames: {
        source: 'not_connected',
        items:
          resolved.configuration.identityAssignment.kind === 'allow_list'
            ? resolved.configuration.identityAssignment.identityIds
            : [],
        reason:
          'Reusable Identity definitions are application-owned; this Harness stores IDs only.',
      },
      agentVisualIdentities: {
        source: 'not_connected',
        items: [],
        reason: 'Identity colors and shapes are loaded from the application Identity catalog.',
      },
      skills: {
        source: 'checked_in_product_catalog',
        items: resolved.configuration.skills.items.map((item) => ({
          name: item.name,
          path: item.path,
          description: item.purpose,
          text: null,
        })),
        reason: 'Published Harness skill references.',
      },
      tools: {
        source: 'recorded_harness_tool_catalog',
        items: resolved.configuration.tools.items.map(({ name }) => ({ name, description: '' })),
        reason: 'Published Harness tools and MCP exposure.',
      },
      models: {
        source: 'not_connected',
        items: modelItems,
        reason: 'The model catalog is application-global; the Harness stores only a preference.',
      },
    },
    workingCopy: details.draft
      ? {
          baseRevision: details.draft.basedOn?.version ?? 0,
          draftRevision: (details.draft.basedOn?.version ?? 0) + 1,
          dirty: true,
          configuration: toEditorConfiguration(details.harness.name, details.draft.configuration),
        }
      : null,
    sessionWorkingCopy: sessionOverrideDraft
      ? {
          baseRevision: sessionOverrideDraft.baseHarnessRef.version,
          dirty: true,
          configuration: toEditorConfiguration(
            details.harness.name,
            sessionOverrideDraft.configuration,
          ),
        }
      : null,
    versionControl: {
      support: 'recorded_preview',
      pushedRevision: pushedVersions.size > 0 ? Math.max(...pushedVersions) : null,
      versions: details.versions.map((version) => ({
        revision: version.reference.version,
        label: `Version ${version.reference.version}`,
        status: pushedVersions.has(version.reference.version)
          ? 'pushed'
          : replacedVersions.has(version.reference.version)
            ? 'committed'
            : 'committed',
        configuration: toEditorConfiguration(details.harness.name, version.configuration),
        activeSessionCount: 0,
        queuedSessionCount: 0,
        committedAt: version.createdAt,
      })),
      reason: 'Immutable Harness versions managed by the application Harness engine.',
    },
    sessionBinding: {
      state:
        requested.version === resolved.reference.version &&
        requested.harnessId === resolved.reference.harnessId
          ? 'current'
          : 'behind',
      appliedRevision: requested.version,
      desiredRevision: resolved.reference.version,
      relevantSessionCount: null,
      executingPreviousInvocation: false,
      reason:
        requested.version === resolved.reference.version
          ? 'This Session owns the current resolved Harness version.'
          : 'The Harness engine will migrate this Session reference before its next launch.',
    },
    modelChoices: {
      delegatedPolicies: [],
      sessionOverride: session.requestedOptions.model
        ? { model: session.requestedOptions.model, reasoning: null }
        : null,
      userPreference: {
        support: 'not_connected',
        lastUsedModel: null,
        lastUsedReasoning: null,
        reason: 'Application-global model preferences are managed in Technical Settings.',
      },
      resolvedForCurrentSession: {
        model: session.requestedOptions.model ?? preferredModel?.modelId ?? null,
        reasoning: preferredModel?.reasoning ?? null,
        source: session.requestedOptions.model
          ? 'session_override'
          : preferredModel
            ? 'harness_revision'
            : 'not_connected',
      },
    },
  };
}

function versionRef(context: AvailableContext, version: number): HarnessVersionRef {
  const reference = createHarnessVersionRef(
    context.details.harness.harnessId,
    createHarnessVersionNumber(version),
  );
  if (!context.details.versions.some((candidate) => candidate.reference.version === version))
    throw new Error(`Harness version ${version} does not exist.`);
  return reference;
}

function configurationForVersion(
  context: AvailableContext,
  reference: HarnessVersionRef,
): HarnessConfiguration {
  const version = context.details.versions.find(
    (candidate) =>
      candidate.reference.harnessId === reference.harnessId &&
      candidate.reference.version === reference.version,
  );
  if (!version) throw new Error(`Harness version ${reference.version} does not exist.`);
  return version.configuration;
}

function requireSessionOverrideDraft(
  drafts: SessionHarnessOverrideDraftCache,
  sessionId: string,
): SessionHarnessOverrideDraft {
  const draft = drafts.get(sessionId);
  if (!draft) throw new Error('Customize this Session before saving or publishing.');
  return draft;
}

function toEditorConfiguration(
  name: string,
  configuration: HarnessConfiguration,
): HarnessEffectiveConfiguration {
  const preference = configuration.runtime.preferredModel;
  return {
    identity: {
      name,
      machineKey: '',
      permittedAgentNames:
        configuration.identityAssignment.kind === 'allow_list'
          ? configuration.identityAssignment.identityIds
          : null,
      visualIdentity: null,
    },
    promptPrefix: configuration.promptPrefix,
    skills: configuration.skills,
    tools: configuration.tools,
    runtime: {
      modelPolicyMode: 'revision_owned',
      models: preference
        ? [
            {
              modelId: preference.modelId,
              allowed: true,
              minReasoning: preference.reasoning ?? 'low',
              maxReasoning: preference.reasoning ?? 'xhigh',
            },
          ]
        : [],
      defaultModel: preference?.modelId ?? null,
      defaultReasoning: preference?.reasoning ?? null,
      sandbox: configuration.runtime.sandbox,
      sandboxOptions: [configuration.runtime.sandbox],
      approvalPolicy: configuration.runtime.approvalPolicy,
      approvalPolicyOptions: [configuration.runtime.approvalPolicy],
      authoritySummary: configuration.runtime.authoritySummary,
    },
    hooks: configuration.hooks,
    updatePolicy: configuration.updatePolicy,
  };
}

function toCanonicalConfiguration(
  configuration: HarnessEffectiveConfiguration,
  baseline: HarnessConfiguration,
): HarnessConfiguration {
  const defaultReasoning = configuration.runtime.defaultReasoning as HarnessReasoningLevel | null;
  return {
    identityAssignment:
      configuration.identity.permittedAgentNames === null
        ? { kind: 'unrestricted' }
        : { kind: 'allow_list', identityIds: configuration.identity.permittedAgentNames },
    promptPrefix: configuration.promptPrefix,
    skills: configuration.skills,
    tools: {
      ...configuration.tools,
      mcpServers: configuration.tools.mcpServers ?? baseline.tools.mcpServers,
    },
    runtime: {
      preferredModel: configuration.runtime.defaultModel
        ? { modelId: configuration.runtime.defaultModel, reasoning: defaultReasoning }
        : null,
      sandbox: configuration.runtime.sandbox,
      approvalPolicy: configuration.runtime.approvalPolicy,
      authoritySummary: configuration.runtime.authoritySummary,
    },
    hooks: configuration.hooks,
    updatePolicy: configuration.updatePolicy,
  };
}

class UnboundSessionHarness extends Error {}
