import type { HarnessConfiguration } from './configuration';

export function exampleHarnessConfiguration(): HarnessConfiguration {
  return {
    identityAssignment: {
      kind: 'allow_list',
      identityIds: ['identity-avery'],
    },
    promptPrefix: {
      content: 'Build an Epic plan.',
      initialDelivery: 'prepend',
      contextCompressionDelivery: 'deferred',
    },
    skills: {
      availableDiscoveryPolicy: 'whitelist',
      items: [
        {
          name: 'epic-plan-builder',
          path: '.agents/skills/epic-plan-builder',
          purpose: 'Build an Epic plan.',
          useWhen: 'The user asks for an Epic plan.',
          policy: 'available',
        },
      ],
    },
    tools: {
      availableDiscoveryPolicy: 'whitelist',
      items: [{ name: 'submit_epic_plan', policy: 'available' }],
      schemaBoundary: 'product_owned',
      mcpServers: [{ serverName: 'workflow-tools', access: { kind: 'entire_server' } }],
    },
    runtime: {
      preferredModel: { modelId: 'gpt-5.6-terra', reasoning: 'high' },
      sandbox: 'workspace_write',
      approvalPolicy: 'never',
      authoritySummary: 'Application user has full authority.',
    },
    hooks: [],
    updatePolicy: {
      status: 'configured',
      delivery: 'next_prompt',
      avoidDuplicateGuidance: true,
      notifyRemovedItems: true,
      promptReconstruction: 'deferred',
    },
  };
}
