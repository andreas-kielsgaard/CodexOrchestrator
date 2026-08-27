import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { useState } from 'react';
import type {
  HarnessConfigurationCatalogs,
  HarnessEffectiveConfiguration,
} from '../../application/conversationHarnesses';
import { HarnessDefinitionEditor } from './HarnessDefinitionEditor';

describe('HarnessDefinitionEditor', () => {
  it('edits the same detached definition contract in Session and Role surfaces', async () => {
    function SharedContractFixture() {
      const [sessionDefinition, setSessionDefinition] = useState(definition('Session Harness'));
      const [roleDefinition, setRoleDefinition] = useState(definition('Role Harness'));
      return (
        <>
          <section aria-label="Session Harness definition">
            <HarnessDefinitionEditor
              configuration={sessionDefinition}
              catalogs={catalogs}
              editable
              onChange={setSessionDefinition}
            />
          </section>
          <section aria-label="Role Harness definition">
            <HarnessDefinitionEditor
              configuration={roleDefinition}
              catalogs={catalogs}
              editable
              onChange={setRoleDefinition}
            />
          </section>
        </>
      );
    }

    render(<SharedContractFixture />);
    const session = screen.getByRole('region', { name: 'Session Harness definition' });
    const role = screen.getByRole('region', { name: 'Role Harness definition' });
    expect(within(session).getByTestId('harness-definition-editor')).toBeVisible();
    expect(within(role).getByTestId('harness-definition-editor')).toBeVisible();
    expect(within(session).getByLabelText('Harness name')).toHaveValue('Session Harness');
    expect(within(role).getByLabelText('Harness name')).toHaveValue('Role Harness');
    expect(within(session).queryByLabelText('Harness machine key')).toBeNull();
    expect(within(role).queryByText('Machine key')).toBeNull();

    fireEvent.change(within(role).getByLabelText('Harness name'), {
      target: { value: 'Security Harness' },
    });
    expect(within(role).getByLabelText('Harness name')).toHaveValue('Security Harness');
    expect(within(session).getByLabelText('Harness name')).toHaveValue('Session Harness');
  });

  it('uses searchable keyboard-operable selectors for closed single and multi values', async () => {
    const onChange = vi.fn();
    render(
      <HarnessDefinitionEditor
        configuration={definition('Role Harness')}
        catalogs={catalogs}
        editable
        mcpComponents={[{ serverName: 'workflow', toolName: 'handoff', title: 'Workflow handoff' }]}
        onChange={onChange}
      />,
    );

    const sandbox = screen.getByRole('combobox', { name: 'Sandbox' });
    expect(sandbox.closest('label')).toBeNull();
    fireEvent.focus(sandbox);
    expect(sandbox).toHaveAttribute('aria-expanded', 'true');
    fireEvent.keyDown(sandbox, { key: 'ArrowDown' });
    await waitFor(() => expect(screen.getByRole('option', { name: 'Read only' })).toHaveFocus());
    expect(screen.getByRole('option', { name: 'Read only' }).closest('label')).toBeNull();
    fireEvent.keyDown(screen.getByRole('option', { name: 'Read only' }), { key: 'End' });
    expect(screen.getByRole('option', { name: 'Danger full access' })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole('option', { name: 'Danger full access' }), {
      key: 'Escape',
    });
    expect(sandbox).toHaveFocus();
    expect(sandbox).toHaveAttribute('aria-expanded', 'false');

    const tools = screen.getByRole('combobox', { name: 'Harness tools' });
    fireEvent.focus(tools);
    fireEvent.change(tools, { target: { value: 'handoff' } });
    const tool = screen.getByRole('option', { name: /handoff/i });
    fireEvent.click(tool);
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({
        tools: expect.objectContaining({
          items: [expect.objectContaining({ name: 'handoff' })],
        }),
      }),
    );
  });

  it('shows unavailable catalogs instead of accepting arbitrary closed-set text', () => {
    const baseDefinition = definition('Role Harness');
    const configuredDefinition: HarnessEffectiveConfiguration = {
      ...baseDefinition,
      skills: {
        ...baseDefinition.skills,
        items: [
          {
            name: 'review',
            path: 'skills/review',
            purpose: 'Review changes.',
            useWhen: 'A review is requested.',
            policy: 'available',
          },
        ],
      },
      tools: {
        ...baseDefinition.tools,
        mcpServers: [
          {
            serverName: 'workflow',
            access: { kind: 'selected_tools', toolNames: ['handoff'] },
          },
        ],
      },
    };
    render(
      <HarnessDefinitionEditor
        configuration={configuredDefinition}
        catalogs={{
          ...catalogs,
          agentNames: { source: 'not_connected', items: [], reason: 'Name catalog unavailable.' },
          skills: { source: 'not_connected', items: [], reason: 'Skill catalog unavailable.' },
        }}
        editable
        onChange={() => undefined}
      />,
    );

    expect(screen.getByText('Name catalog unavailable.')).toHaveAttribute('role', 'status');
    expect(screen.getByText('Skill catalog unavailable.')).toHaveAttribute('role', 'status');
    expect(screen.queryByRole('combobox', { name: 'Permitted Agent names' })).toBeNull();
    expect(screen.queryByRole('combobox', { name: 'Harness skills' })).toBeNull();
    expect(screen.getByText('review')).toBeVisible();
    expect(screen.getByText('workflow: handoff')).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Workflow MCP exposure' })).toBeNull();
    expect(screen.getByRole('combobox', { name: 'Model policy ownership' })).toBeVisible();
    expect(screen.getByRole('combobox', { name: 'Harness default model' })).toBeVisible();
  });

  it('keeps selected values inspectable when a connected catalog is only partial', () => {
    const baseDefinition = definition('Role Harness');
    const configuredDefinition: HarnessEffectiveConfiguration = {
      ...baseDefinition,
      skills: {
        ...baseDefinition.skills,
        items: [
          {
            name: 'retired-review',
            path: 'skills/retired-review',
            purpose: 'Recorded selection.',
            useWhen: 'Recorded selection.',
            policy: 'available',
          },
        ],
      },
      tools: {
        ...baseDefinition.tools,
        mcpServers: [
          {
            serverName: 'retired-server',
            access: { kind: 'selected_tools', toolNames: ['retired-tool'] },
          },
        ],
      },
    };
    render(
      <HarnessDefinitionEditor
        configuration={configuredDefinition}
        catalogs={catalogs}
        editable
        mcpComponents={[{ serverName: 'workflow', toolName: 'handoff', title: 'Workflow handoff' }]}
        onChange={() => undefined}
      />,
    );

    fireEvent.focus(screen.getByRole('combobox', { name: 'Harness skills' }));
    expect(screen.getByRole('option', { name: 'retired-review' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    fireEvent.focus(screen.getByRole('combobox', { name: 'Workflow MCP exposure' }));
    expect(screen.getByRole('option', { name: /retired-server \/ retired-tool/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });
});

const catalogs: HarnessConfigurationCatalogs = {
  agentNames: {
    source: 'product_default_pool',
    items: ['Avery', 'Riley'],
    reason: 'Recorded names.',
  },
  agentVisualIdentities: {
    source: 'product_visual_catalog',
    items: [
      {
        identity: { token: 'sunflower', accent: '#f7bd3f', shape: 'circle' },
        label: 'Sunflower',
      },
      { identity: { token: 'ocean', accent: '#287fbc', shape: 'square' }, label: 'Ocean' },
    ],
    reason: 'Recorded identities.',
  },
  skills: {
    source: 'checked_in_product_catalog',
    items: [{ name: 'review', path: 'skills/review', description: 'Review changes.', text: null }],
    reason: 'Checked-in skills.',
  },
  tools: {
    source: 'recorded_harness_tool_catalog',
    items: [{ name: 'handoff', description: 'Hand work to another Agent.' }],
    reason: 'Recorded tools.',
  },
  models: {
    source: 'recorded_catalog',
    items: [
      {
        id: 'gpt-5.6-terra',
        label: 'GPT-5.6 Terra',
        reasoningLevels: ['low', 'medium', 'high', 'xhigh'],
      },
    ],
    reason: 'Recorded models.',
  },
};

function definition(name: string): HarnessEffectiveConfiguration {
  return {
    identity: {
      name,
      machineKey: name.toLowerCase().replaceAll(' ', '_'),
      permittedAgentNames: null,
      visualIdentity: null,
    },
    promptPrefix: {
      content: 'Review the request.',
      initialDelivery: 'prepend',
      contextCompressionDelivery: 'deferred',
    },
    skills: { availableDiscoveryPolicy: 'whitelist', items: [] },
    tools: {
      availableDiscoveryPolicy: 'whitelist',
      items: [],
      schemaBoundary: 'Schemas remain runtime-owned.',
      mcpServers: [],
    },
    runtime: {
      modelPolicyMode: 'revision_owned',
      models: [
        {
          modelId: 'gpt-5.6-terra',
          allowed: true,
          minReasoning: 'low',
          maxReasoning: 'xhigh',
        },
      ],
      defaultModel: null,
      defaultReasoning: null,
      sandbox: 'workspace_write',
      sandboxOptions: ['read_only', 'workspace_write', 'danger_full_access'],
      approvalPolicy: 'never',
      approvalPolicyOptions: ['never'],
      authoritySummary: '',
    },
    hooks: [],
    updatePolicy: { status: 'not_configured', reason: 'Not configured.' },
  };
}
