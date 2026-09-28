import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type { ExecutionBindingDto } from '../../application/executionTargets/contracts';
import { capabilityProfileDeviceModels, withCapabilityProfileDeviceModels } from './routeModels';

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

it('offers the capability profile models on the same device without exposing route composition', () => {
  const models = capabilityProfileDeviceModels(profile, binding('codex'));
  expect(models.map((model) => model.id)).toEqual(['gpt-5', 'opus']);
  expect(models[1].label).toBe('opus');
  expect(models[1].reasoningModes.map((mode) => mode.id)).toEqual(['low', 'medium', 'high']);
  const merged = withCapabilityProfileDeviceModels(
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

it('uses provider-observed models even when no model preferences were recorded', () => {
  const claude = route('claude', 'ignored');
  const withoutPreferences = {
    ...profile,
    routePolicies: [{ ...claude, modelAllowances: [] }],
  } as unknown as CapabilityProfileDto;
  const models = capabilityProfileDeviceModels(withoutPreferences, binding('claude'), {
    'local/claude/claude-setup': {
      route: {
        deviceId: 'local',
        provider: 'claude',
        configurationRef: 'claude-setup',
      },
      observedAt: '2026-09-28T00:00:00Z',
      observationError: null,
      models: [
        {
          id: 'claude-haiku-4-5',
          label: 'Haiku',
          description: 'Fast',
          defaultReasoningMode: 'high',
          reasoningModes: [{ id: 'high', description: '' }],
        },
      ],
    },
  });

  expect(models.map((model) => model.id)).toEqual(['claude-haiku-4-5']);
  expect(models[0].label).toBe('Haiku');
});
