import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type { ExecutionBindingDto } from '../../application/executionTargets/contracts';
import { otherRouteModels, withOtherRouteModels } from './routeModels';

const binding = (provider: string, deviceId = 'local'): ExecutionBindingDto => ({
  deviceId,
  deviceName: deviceId,
  provider,
  configurationRef: `${provider}-setup`,
  connection: { kind: 'local' },
});

const route = (provider: string, modelId: string, deviceId = 'local') => ({
  routeId: `${deviceId}-${provider}`,
  execution: binding(provider, deviceId),
  modelAllowances: [{ modelId, minimumReasoning: 'low', maximumReasoning: 'high' }],
  mcpGroups: [],
  skillGroups: [],
  defaults: { model: null, reasoningMode: null, sandboxMode: null },
});

const profile = {
  routePolicies: [
    route('codex', 'gpt-5'),
    route('claude', 'opus'),
    route('claude', 'sonnet', 'server'),
  ],
} as unknown as CapabilityProfileDto;

it('offers the other providers models on the same device', () => {
  const models = otherRouteModels(profile, binding('codex'));
  expect(models.map((model) => model.id)).toEqual(['opus']);
  expect(models[0].label).toBe('opus · Claude');
  expect(models[0].reasoningModes.map((mode) => mode.id)).toEqual(['low', 'medium', 'high']);
  const merged = withOtherRouteModels(
    {
      configuration: null,
      models: [
        {
          id: 'gpt-5',
          label: 'gpt-5',
          description: '',
          defaultReasoningMode: null,
          reasoningModes: [],
        },
      ],
      skills: [],
      defaults: { model: null, reasoningMode: null },
      limitations: [],
    },
    models,
  );
  expect(merged?.models.map((model) => model.id)).toEqual(['gpt-5', 'opus']);
});
