import type { AgentProviderDescriptor } from '../contracts';

export const claudeProviderDescriptor: AgentProviderDescriptor = {
  id: 'claude',
  label: 'Claude',
  configurationLabel: 'Claude setup',
  harnessLabel: 'Claude Code CLI',
  inferenceLabel: 'Anthropic account via Claude Code CLI',
};
