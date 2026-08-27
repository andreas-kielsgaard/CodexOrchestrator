import type { HarnessMetadata } from './configuration';
import { exampleHarnessConfiguration } from './testFixtures';

describe('Harness configuration contracts', () => {
  it('keeps metadata separate from versioned behavior', () => {
    const metadata: HarnessMetadata = { name: 'Epic Plan Builder' };
    const configuration = exampleHarnessConfiguration();

    expect(metadata).toEqual({ name: 'Epic Plan Builder' });
    expect(configuration).not.toHaveProperty('name');
    expect(configuration).not.toHaveProperty('machineKey');
    expect(configuration.runtime).not.toHaveProperty('models');
    expect(configuration.runtime).not.toHaveProperty('modelPolicyMode');
  });

  it('represents identity scoping, global model preference, and MCP exposure explicitly', () => {
    const configuration = exampleHarnessConfiguration();

    expect(configuration.identityAssignment).toEqual({
      kind: 'allow_list',
      identityIds: ['identity-avery'],
    });
    expect(configuration.runtime.preferredModel).toEqual({
      modelId: 'gpt-5.6-terra',
      reasoning: 'high',
    });
    expect(configuration.tools.mcpServers).toEqual([
      { serverName: 'workflow-tools', access: { kind: 'entire_server' } },
    ]);
  });
});
