import { createRoot } from 'react-dom/client';
import { WorkflowAuthoringScreen } from '../../../src/features/workflowAuthoring/WorkflowAuthoringScreen';
import { ExecutionConfigurationScreen } from '../../../src/features/executionConfiguration/ExecutionConfigurationScreen';
import type { ExecutionConfigurationClient } from '../../../src/application/executionConfiguration';
import type {
  WorkflowAuthoringClient,
  WorkflowRecipeStateDto,
} from '../../../src/application/workflowAuthoring';
import { StandaloneAgentSessionScreen } from '../../../src/features/agentSessions/AgentSessionScreen';
import { sessionDetails, sessionSummary } from '../../../src/features/agentSessions/testFixtures';
import type { AgentSessionClient } from '../../../src/application/agentSessions';
import type { AgentSessionProfileClient } from '../../../src/application/agentSessionProfiles';
import '../../../src/styles.css';

// Real feature components with local fake clients. No Tauri or provider calls.
const clone = <T,>(value: T): T => structuredClone(value);
const runtimeCapabilities = {
  models: ['gpt-5.6-sol', 'gpt-5.6-terra'],
  reasoningModes: ['high', 'low', 'max', 'medium', 'ultra', 'xhigh'],
  sandboxModes: ['workspace_write' as const],
  mcpTools: {},
  skills: [],
};
const allowedCapabilities = { ...runtimeCapabilities, models: ['gpt-5.6-sol'] };
const runtime = {
  contractVersion: 1 as const,
  profileRef: 'review-fixture/runtime',
  exposure: runtimeCapabilities,
  locked: { model: null, reasoningMode: null, sandboxMode: 'workspace_write' as const },
};
const profile = {
  capabilityProfileId: 'review-capabilities',
  name: 'Review capabilities',
  revision: 1,
  allowedCapabilities,
};
const node = (id: string, name: string) => ({
  nodeId: id,
  name,
  positionX: 100,
  positionY: 100,
  capabilityProfileId: profile.capabilityProfileId,
  agentIdentityId: null,
  initialPrompt: 'Review this plan.',
  nodeProfile: {
    contractVersion: 1 as const,
    allowedCapabilities: clone(allowedCapabilities),
    pinnedDefaults: {
      model: 'gpt-5.6-sol',
      reasoningMode: 'high',
      sandboxMode: 'workspace_write' as const,
    },
  },
});
const recipe = (id: string, name: string): WorkflowRecipeStateDto => ({
  draft: {
    contractVersion: 1,
    recipeId: id,
    name,
    revision: 1,
    startingNodeId: `${id}-author`,
    nodes: [node(`${id}-author`, 'Plan Author'), node(`${id}-reviewer`, 'Review Gate')],
    connections: [],
  },
  active: null,
  createdAt: '2026-09-07T00:00:00Z',
  updatedAt: '2026-09-07T00:00:00Z',
});
const states = [recipe('review', 'Review workflow'), recipe('other', 'Other workflow')];
states[0].active = clone(states[0].draft);
const calls: {
  saves: unknown[];
  dispatches: unknown[];
  compiles: unknown[];
  genericSends: unknown[];
  profileLoads: string[];
  profileSends: unknown[];
} = {
  saves: [],
  dispatches: [],
  compiles: [],
  genericSends: [],
  profileLoads: [],
  profileSends: [],
};
Object.assign(window, { audit: { calls, states, runtime, profile } });
const configClient: ExecutionConfigurationClient = {
  loadSelectedRuntimeProfile: async () => clone(runtime),
  listCapabilityProfiles: async () => [clone(profile)],
  loadCapabilityProfile: async () => clone(profile),
  createCapabilityProfile: async (input) => ({ ...input, revision: 1 }),
  updateCapabilityProfile: async (input) => ({ ...input, revision: 2 }),
  deleteCapabilityProfile: async () => {},
};
const client: WorkflowAuthoringClient = {
  listRecipes: async () =>
    states.map(({ draft, active, updatedAt }) => ({
      recipeId: draft.recipeId,
      name: draft.name,
      draftRevision: draft.revision,
      activeRevision: active?.revision ?? null,
      updatedAt,
    })),
  loadRecipe: async (id) => clone(states.find((s) => s.draft.recipeId === id)!),
  createRecipe: async (name) => {
    const state = recipe(`new-${states.length}`, name);
    states.push(state);
    return clone(state);
  },
  saveDraft: async (draft) => {
    calls.saves.push(clone(draft));
    const state = states.find((s) => s.draft.recipeId === draft.recipeId)!;
    state.draft = clone({ ...draft, revision: draft.revision + 1 });
    return clone(state);
  },
  activateRecipe: async (id) => {
    const state = states.find((s) => s.draft.recipeId === id)!;
    state.active = clone(state.draft);
    return clone(state);
  },
  copyNodeConfiguration: async () => {
    throw new Error('Not used by mounted screen');
  },
  compileRecipeInstance: async (recipeId, instanceId) => {
    calls.compiles.push({ recipeId, instanceId });
    return [{} as never];
  },
  dispatchUserRequest: async (input) => {
    calls.dispatches.push(input);
    throw new Error('Provider calls disabled in review fixture');
  },
};
// Mirror the ordinary creation response: a Session exists, but has no pinned profile.
// Completion is immediate and fake. This checks UI routing, not the Rust creation path.
let sessionCreated = false;
const ordinarySessionClient: AgentSessionClient = {
  createSession: async () => {
    sessionCreated = true;
    return sessionDetails().session;
  },
  listSessions: async () => (sessionCreated ? [sessionSummary()] : []),
  loadSession: async () => sessionDetails('completed'),
  reloadSession: async () => sessionDetails('completed'),
  subscribeUpdates: async () => () => {},
  sendMessage: async (input) => {
    calls.genericSends.push(input);
    sessionCreated = true;
    return { sessionId: 'session-1', invocationId: 'invocation-1' };
  },
  cancelInvocation: async () => sessionDetails('canceled').invocations[0].invocation,
  disconnectUpdates: async () => {},
};
const ordinaryProfileClient: AgentSessionProfileClient = {
  loadPinnedProfile: async (id) => {
    calls.profileLoads.push(id);
    throw new Error('Session has no pinned Session Profile.');
  },
  sendDirectUserMessage: async (input) => {
    calls.profileSends.push(input);
    throw new Error('Provider calls disabled in review fixture');
  },
};
const view = new URLSearchParams(location.search).get('view');
createRoot(document.getElementById('root')!).render(
  <div style={{ height: '100vh', display: 'grid', gridTemplateRows: '40px minmax(0, 1fr)' }}>
    <div style={{ padding: '8px 18px', background: '#fff4d6' }}>
      Regression review fixture. Fake data. No provider calls.
    </div>
    {view === 'session' ? (
      <StandaloneAgentSessionScreen
        client={ordinarySessionClient}
        profileClient={ordinaryProfileClient}
      />
    ) : view === 'capabilities' ? (
      <ExecutionConfigurationScreen client={configClient} />
    ) : (
      <WorkflowAuthoringScreen client={client} executionConfigurationClient={configClient} />
    )}
  </div>,
);
