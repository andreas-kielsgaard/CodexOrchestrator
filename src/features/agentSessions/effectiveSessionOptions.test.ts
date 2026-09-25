import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import { effectiveSessionOptions, selectionForModel } from './effectiveSessionOptions';

const capabilities: AgentSessionQuickFeatures = {
  configuration: { provider: 'codex', configurationId: 'selected' },
  defaults: { model: 'astra', reasoningMode: 'high' },
  models: [
    {
      id: 'astra',
      label: 'Astra',
      description: '',
      defaultReasoningMode: 'high',
      reasoningModes: [
        { id: 'low', description: '' },
        { id: 'high', description: '' },
      ],
    },
    {
      id: 'luna',
      label: 'Luna',
      description: '',
      defaultReasoningMode: 'medium',
      reasoningModes: [{ id: 'medium', description: '' }],
    },
  ],
  skills: [],
  limitations: [],
};

it('always resolves concrete available defaults', () => {
  expect(effectiveSessionOptions(capabilities, { model: null, reasoningMode: null })).toEqual({
    model: capabilities.models[0],
    reasoningMode: 'high',
  });
});

it('moves an incompatible reasoning choice to the selected model default', () => {
  expect(selectionForModel(capabilities, { model: 'astra', reasoningMode: 'low' }, 'luna')).toEqual(
    { model: 'luna', reasoningMode: 'medium' },
  );
});
