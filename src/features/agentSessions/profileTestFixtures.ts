import type { AgentSessionClient } from '../../application/agentSessions';
import type {
  AgentSessionProfileClient,
  PinnedAgentSessionProfileDto,
  SendDirectUserAgentSessionMessageResultDto,
} from '../../application/agentSessions';
import { repairRuntime, repairProfile } from '../workflowAuthoring/testFixtures';
import { sessionDetails, sessionSummary } from './testFixtures';

export function repairSessionClients(initiallyCreated = true) {
  let created = initiallyCreated;
  const details = sessionDetails('completed');
  details.session.assignedIdentity = {
    originIdentityId: null,
    displayName: 'Avery',
    color: '#39745a',
    shape: 'circle',
  };
  const profile: PinnedAgentSessionProfileDto = {
    sessionId: details.session.id,
    creationResolution: {
      contractVersion: 1,
      digest: 'fixture-digest',
      sessionProfile: {
        contractVersion: 1,
        configuration: repairRuntime.configuration,
        attachedRuntimeCapabilities: repairRuntime.exposure,
        attachedRuntimeLocked: repairRuntime.locked,
        capabilityProfileId: repairProfile.capabilityProfileId,
        capabilityProfileRevision: 1,
        nodeCapabilities: repairProfile.allowedCapabilities,
        pinnedDefaults: { model: 'model-a', reasoningMode: 'high', sandboxMode: 'workspace_write' },
      },
    },
  };
  const result: SendDirectUserAgentSessionMessageResultDto = {
    sessionId: details.session.id,
    invocationId: 'invocation-next',
    invocationResolution: {
      contractVersion: 1,
      sessionProfileDigest: 'fixture-digest',
      selections: profile.creationResolution.sessionProfile.pinnedDefaults,
    },
  };
  const sessions: AgentSessionClient = {
    listSessions: async () => (created ? [sessionSummary()] : []),
    loadSession: async () => structuredClone(details),
    reloadSession: async () => structuredClone(details),
    createSession: async () => {
      throw new Error('Use the profiled birth route.');
    },
    sendMessage: async () => {
      throw new Error('Use the profiled send route.');
    },
    subscribeUpdates: async () => () => {},
    disconnectUpdates: async () => {},
    cancelInvocation: async () => details.invocations[0].invocation,
    updateIdentity: async ({ assignedIdentity }) => {
      details.session.assignedIdentity = assignedIdentity;
      return structuredClone(details.session);
    },
  };
  const profiles: AgentSessionProfileClient = {
    loadQuickFeatures: async () => ({
      configuration: repairRuntime.configuration,
      defaults: profile.creationResolution.sessionProfile.pinnedDefaults,
      models: repairRuntime.exposure.models.map((id) => ({
        id,
        label: id,
        description: '',
        defaultReasoningMode: 'medium',
        reasoningModes: repairRuntime.exposure.reasoningModes.map((id) => ({
          id,
          description: '',
        })),
      })),
      skills: [
        { id: 'review', name: 'review', description: 'Review changes', invocationText: '$review' },
      ],
      limitations: [],
    }),
    loadPinnedProfile: async () => structuredClone(profile),
    startDirectUserSession: async () => {
      created = true;
      return structuredClone(result);
    },
    sendDirectUserMessage: async () => structuredClone(result),
  };
  return { sessions, profiles, profile, details, result };
}
