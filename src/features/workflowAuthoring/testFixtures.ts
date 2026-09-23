import catalogue from './otpCatalogue.fixture.json';
import type { OtpPackageDto } from '../../application/workflowAuthoring';
export const otpCatalogue = catalogue as unknown as readonly OtpPackageDto[];
import type {
  CapabilityProfileDto,
  ExecutionConfigurationClient,
  RuntimeProfileSnapshotDto,
} from '../../application/executionConfiguration';
import type {
  WorkflowAuthoringClient,
  WorkflowRecipeStateDto,
} from '../../application/workflowAuthoring';

export const repairRuntime: RuntimeProfileSnapshotDto = {
  contractVersion: 1,
  profileRef: 'fixture/runtime',
  exposure: {
    models: ['model-a', 'model-b'],
    reasoningModes: ['high', 'medium'],
    sandboxModes: ['workspace_write'],
    mcpTools: {},
    skills: [],
  },
  locked: { model: null, reasoningMode: null, sandboxMode: 'workspace_write' },
};
export const repairProfile: CapabilityProfileDto = {
  contractVersion: 1,
  capabilityProfileId: 'review',
  name: 'Review capabilities',
  revision: 1,
  allowedCapabilities: { ...repairRuntime.exposure, models: ['model-a'] },
  execution: {
    deviceId: 'fixture-device',
    deviceName: 'Fixture device',
    provider: 'codex',
    configurationRef: 'fixture-codex',
    connection: { kind: 'local' },
  },
  routePolicies: [
    {
      routeId: 'fixture-default',
      execution: {
        deviceId: 'fixture-device',
        deviceName: 'Fixture device',
        provider: 'codex',
        configurationRef: 'fixture-codex',
        connection: { kind: 'local' },
      },
      modelAllowances: [
        { modelId: 'model-a', minimumReasoning: 'medium', maximumReasoning: 'high' },
      ],
      mcpGroups: [],
      skillGroups: [],
      defaults: { model: null, reasoningMode: null, sandboxMode: null },
    },
  ],
  defaultRouteId: 'fixture-default',
};
export function repairRecipe(id = 'review', name = 'Review workflow'): WorkflowRecipeStateDto {
  const draft: WorkflowRecipeStateDto['draft'] = {
    contractVersion: 2,
    entryAction: { package: 'workflow', tool: 'prompt_agent' },
    recipeId: id,
    name,
    revision: 1,
    startingNodeId: 'author',
    nodes: ['author', 'reviewer'].map((nodeId, index) => ({
      nodeId,
      name: index ? 'Reviewer' : 'Author',
      positionX: 60 + index * 420,
      positionY: 100,
      capabilityProfileId: repairProfile.capabilityProfileId,
      initialPrompt: `Initial for ${nodeId}`,
      agentIdentityId: null,
      nodeProfile: {
        contractVersion: 1,
        allowedCapabilities: structuredClone(repairProfile.allowedCapabilities),
        pinnedDefaults: { model: 'model-a', reasoningMode: 'high', sandboxMode: 'workspace_write' },
      },
    })),
    connections: [],
  };
  return {
    draft,
    active: structuredClone(draft),
    createdAt: '2026-09-07',
    updatedAt: '2026-09-07',
  };
}

/** Real UI, local in-memory clients only. Shared by component and browser regression checks. */
export function repairClients() {
  const states = [repairRecipe(), repairRecipe('other', 'Other workflow')];
  const profiles = [
    structuredClone(repairProfile),
    { ...structuredClone(repairProfile), capabilityProfileId: 'other', name: 'Other capabilities' },
  ];
  const configuration: ExecutionConfigurationClient = {
    loadSelectedRuntimeProfile: async () => structuredClone(repairRuntime),
    listCapabilityProfiles: async () => structuredClone(profiles),
    loadCapabilityProfile: async (id) =>
      structuredClone(profiles.find((profile) => profile.capabilityProfileId === id)!),
    createCapabilityProfile: async (input) => {
      const saved: CapabilityProfileDto = {
        ...input,
        capabilityProfileId: 'created-profile',
        contractVersion: 1,
        revision: 1,
      };
      profiles.push(saved);
      return structuredClone(saved);
    },
    updateCapabilityProfile: async (input) => {
      const index = profiles.findIndex(
        (profile) => profile.capabilityProfileId === input.capabilityProfileId,
      );
      const saved: CapabilityProfileDto = {
        ...input,
        contractVersion: 1,
        revision: profiles[index].revision + 1,
      };
      profiles[index] = saved;
      return structuredClone(saved);
    },
    deleteCapabilityProfile: async (id) => {
      profiles.splice(
        profiles.findIndex((profile) => profile.capabilityProfileId === id),
        1,
      );
    },
  };
  const authoring: WorkflowAuthoringClient = {
    listRecipes: async () =>
      states.map(({ draft, active, updatedAt }) => ({
        recipeId: draft.recipeId,
        name: draft.name,
        draftRevision: draft.revision,
        activeRevision: active?.revision ?? null,
        updatedAt,
      })),
    loadRecipe: async (id) => structuredClone(states.find((state) => state.draft.recipeId === id)!),
    createRecipe: async (name) => {
      const state = repairRecipe(crypto.randomUUID(), name);
      states.push(state);
      return structuredClone(state);
    },
    saveDraft: async (draft) => {
      const index = states.findIndex((state) => state.draft.recipeId === draft.recipeId);
      states[index] = {
        ...states[index],
        draft: { ...structuredClone(draft), revision: draft.revision + 1 },
      };
      return structuredClone(states[index]);
    },
    activateRecipe: async (id) => {
      const index = states.findIndex((state) => state.draft.recipeId === id);
      states[index] = { ...states[index], active: structuredClone(states[index].draft) };
      return structuredClone(states[index]);
    },
    copyNodeConfiguration: async () => {
      throw new Error('Copy edits the local draft.');
    },
    compileRecipeInstance: async () => {
      throw new Error('Use a stored instance.');
    },
    dispatchUserRequest: async () => {
      throw new Error('Use a stored instance.');
    },
  };
  return { states, profiles, configuration, authoring };
}
