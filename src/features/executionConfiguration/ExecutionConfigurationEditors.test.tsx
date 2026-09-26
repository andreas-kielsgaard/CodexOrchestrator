import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { CapabilityProfileEditor } from './CapabilityProfileEditor';
import { NodeProfileEditor } from './NodeProfileEditor';
import { SessionProfileInspector } from './SessionProfileInspector';
import type {
  CapabilityProfileDraft,
  CapabilitySetViewModel,
  NodeProfileEditorValue,
  RuntimeProfileViewModel,
  SessionProfileViewModel,
} from './types';
import { mcpToolCatalogValue } from './types';

const emptyCapabilities: CapabilitySetViewModel = {
  models: [],
  reasoningModes: [],
  sandboxModes: [],
  mcpTools: {},
  skills: [],
};

const runtime: RuntimeProfileViewModel = {
  configuration: { provider: 'codex', configurationId: 'global' },
  sourceLabel: 'Selected Codex profile',
  exposure: {
    models: ['gpt-5.6', 'gpt-5.4'],
    reasoningModes: ['medium', 'high'],
    sandboxModes: ['workspace_write'],
    mcpTools: { orchestrator: ['session-message'] },
    skills: ['maintain-slice-plan'],
  },
  catalogs: {
    models: {
      availability: 'available',
      sourceLabel: 'Selected Codex profile',
      options: [
        { value: 'gpt-5.6', label: 'GPT 5.6' },
        { value: 'gpt-5.4', label: 'GPT 5.4' },
      ],
    },
    reasoningModes: {
      availability: 'available',
      sourceLabel: 'Selected Codex profile',
      options: [
        { value: 'medium', label: 'Medium' },
        { value: 'high', label: 'High' },
      ],
    },
    sandboxModes: {
      availability: 'available',
      sourceLabel: 'Selected Codex profile',
      options: [{ value: 'workspace_write', label: 'Workspace write' }],
    },
    mcpTools: {
      availability: 'available',
      sourceLabel: 'Orchestrator',
      options: [
        {
          value: mcpToolCatalogValue('orchestrator', 'session-message'),
          label: 'Message Session',
        },
      ],
    },
    skills: {
      availability: 'available',
      sourceLabel: 'Selected Codex profile',
      options: [{ value: 'maintain-slice-plan', label: 'Maintain slice plan' }],
    },
  },
  lockedSelections: { model: null, reasoningMode: null, sandboxMode: 'workspace_write' },
};

function ControlledCapabilityEditor({
  onSave,
  runtimeProfile = runtime,
}: {
  onSave(profile: CapabilityProfileDraft): void;
  runtimeProfile?: RuntimeProfileViewModel;
}) {
  const [profile, setProfile] = useState<CapabilityProfileDraft>({
    capabilityProfileId: '',
    name: '',
    revision: null,
    allowedCapabilities: emptyCapabilities,
    routePolicies: [],
    defaultRouteId: null,
  });
  return (
    <CapabilityProfileEditor
      profile={profile}
      runtime={runtimeProfile}
      routes={[
        {
          id: 'local-codex:review',
          selected: true,
          label: 'This device · Codex CLI',
          sourceLabel: 'OpenAI via Codex CLI',
          detail: 'C:/codex-review',
          execution: {
            deviceId: 'local',
            deviceName: 'This device',
            provider: 'codex',
            configurationRef: 'review',
            connection: { kind: 'local' },
          },
        },
      ]}
      onChange={setProfile}
      onSave={onSave}
    />
  );
}

describe('Execution Configuration editors', () => {
  it('edits and submits a minimal Capability Profile through controlled props', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(<ControlledCapabilityEditor onSave={onSave} />);

    await user.type(screen.getByRole('textbox', { name: 'Capability profile name' }), 'Reviewer');
    await user.click(screen.getByRole('button', { name: 'Add execution route' }));
    await user.click(screen.getByRole('button', { name: 'Add route' }));
    await user.click(
      screen.getByRole('button', {
        name: 'This device Configured harness OpenAI via Codex CLI',
      }),
    );
    await user.click(screen.getByRole('button', { name: 'Add model' }));
    await user.click(
      within(screen.getByText('gpt-5.6').closest('li') as HTMLElement).getByRole('button', {
        name: 'Add',
      }),
    );
    await user.click(screen.getByRole('button', { name: 'Create profile' }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        name: 'Reviewer',
        execution: {
          deviceId: 'local',
          deviceName: 'This device',
          provider: 'codex',
          configurationRef: 'review',
          connection: { kind: 'local' },
        },
        routePolicies: expect.arrayContaining([
          expect.objectContaining({
            modelAllowances: expect.arrayContaining([
              expect.objectContaining({ modelId: 'gpt-5.6' }),
            ]),
          }),
        ]),
      }),
    );
  });

  it('adds a model that reports no reasoning levels without a range', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(
      <ControlledCapabilityEditor
        onSave={onSave}
        runtimeProfile={{
          ...runtime,
          exposure: { ...runtime.exposure, reasoningModes: [] },
          catalogs: {
            ...runtime.catalogs,
            reasoningModes: { ...runtime.catalogs.reasoningModes, options: [] },
          },
        }}
      />,
    );

    await user.type(screen.getByRole('textbox', { name: 'Capability profile name' }), 'Quick');
    await user.click(screen.getByRole('button', { name: 'Add execution route' }));
    await user.click(screen.getByRole('button', { name: 'Add route' }));
    await user.click(
      screen.getByRole('button', {
        name: 'This device Configured harness OpenAI via Codex CLI',
      }),
    );
    await user.click(screen.getByRole('button', { name: 'Add model' }));
    const add = within(screen.getByText('gpt-5.6').closest('li') as HTMLElement).getByRole(
      'button',
      { name: 'Add' },
    );
    expect(add).toBeEnabled();
    await user.click(add);
    await user.click(screen.getByRole('button', { name: 'Close Add model' }));
    expect(screen.getByText('No reasoning levels')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Create profile' }));

    const saved = onSave.mock.calls[0][0] as CapabilityProfileDraft;
    expect(saved.routePolicies[0].modelAllowances).toEqual([{ modelId: 'gpt-5.6' }]);
  });

  it('keeps the model picker open and restores a removed reasoning range in this instance', async () => {
    const user = userEvent.setup();
    render(<ControlledCapabilityEditor onSave={() => undefined} />);

    await user.click(screen.getByRole('button', { name: 'Add execution route' }));
    await user.click(screen.getByRole('button', { name: 'Add route' }));
    await user.click(
      screen.getByRole('button', {
        name: 'This device Configured harness OpenAI via Codex CLI',
      }),
    );
    await user.click(screen.getByRole('button', { name: 'Add model' }));
    const picker = screen.getByRole('dialog', { name: 'Add model' });
    const modelRow = within(picker).getByText('gpt-5.6').closest('li') as HTMLElement;
    await user.click(within(modelRow).getByRole('button', { name: 'Add' }));
    expect(screen.getByRole('dialog', { name: 'Add model' })).toBeVisible();
    expect(within(modelRow).getByRole('button', { name: 'Remove' })).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Close Add model' }));

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'gpt-5.6 lowest reasoning' }),
      'high',
    );
    await user.click(screen.getByRole('button', { name: 'Remove gpt-5.6' }));
    await user.click(screen.getByRole('button', { name: 'Add model' }));
    const reopenedRow = within(screen.getByRole('dialog', { name: 'Add model' }))
      .getByText('gpt-5.6')
      .closest('li') as HTMLElement;
    await user.click(within(reopenedRow).getByRole('button', { name: 'Add' }));
    await user.click(screen.getByRole('button', { name: 'Close Add model' }));

    expect(screen.getByRole('combobox', { name: 'gpt-5.6 lowest reasoning' })).toHaveValue('high');
    expect(screen.getByRole('combobox', { name: 'gpt-5.6 highest reasoning' })).toHaveValue('high');
  });

  it('presents editable MCP and skill groups rather than individual runtime entries', () => {
    render(
      <CapabilityProfileEditor
        profile={{
          capabilityProfileId: 'reviewer',
          name: 'Reviewer',
          revision: 2,
          allowedCapabilities: emptyCapabilities,
          routePolicies: [],
          defaultRouteId: null,
        }}
        runtime={runtime}
        onChange={() => undefined}
      />,
    );

    expect(
      screen.getByText('Add a route to choose where sessions using this profile run.'),
    ).toBeVisible();
  });

  it('keeps node edits controlled and delegates copy semantics to the caller', async () => {
    const user = userEvent.setup();
    const copy = vi.fn();

    function ControlledNode() {
      const [value, setValue] = useState<NodeProfileEditorValue>({
        nodeName: 'Plan reviewer',
        identityId: 'avery',
        capabilityProfileId: 'reviewer',
        initialPrompt: 'Review the proposed plan.',
        exposedCapabilities: {
          ...emptyCapabilities,
          models: ['gpt-5.6'],
          reasoningModes: ['high'],
          sandboxModes: ['workspace_write'],
        },
        pinnedDefaults: {
          model: 'gpt-5.6',
          reasoningMode: 'high',
          sandboxMode: 'workspace_write',
        },
      });
      return (
        <NodeProfileEditor
          value={value}
          capabilityProfiles={[{ id: 'reviewer', label: 'Reviewer', revision: 2 }]}
          identities={[{ id: 'avery', displayName: 'Avery', color: '#39745a', shape: 'hexagon' }]}
          capabilityCatalogs={runtime.catalogs}
          copySources={[
            { nodeId: 'implementer', nodeName: 'Implementer', summary: 'Implementation defaults' },
          ]}
          runtimeLockedSelections={runtime.lockedSelections}
          onChange={setValue}
          onCopyFromNode={copy}
        />
      );
    }

    render(<ControlledNode />);
    expect(screen.getByLabelText('Avery, Agent identity')).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Default sandbox' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Expand Copy node configuration' }));
    const copySection = screen
      .getByRole('heading', { name: 'Copy node configuration' })
      .closest('section');
    expect(copySection).not.toBeNull();
    await user.selectOptions(
      within(copySection as HTMLElement).getByRole('combobox', { name: 'Source node' }),
      'implementer',
    );
    await user.click(
      within(copySection as HTMLElement).getByRole('button', { name: 'Copy configuration' }),
    );
    expect(copy).toHaveBeenCalledWith('implementer');

    await user.clear(screen.getByRole('textbox', { name: 'Node name' }));
    await user.type(screen.getByRole('textbox', { name: 'Node name' }), 'Merge reviewer');
    expect(screen.getByRole('heading', { name: 'Merge reviewer' })).toBeVisible();
  });

  it('presents a pinned Session Profile without editable or initial-prompt controls', async () => {
    const user = userEvent.setup();
    const profile: SessionProfileViewModel = {
      configuration: runtime.configuration,
      capabilityProfileId: 'reviewer',
      capabilityProfileRevision: 2,
      attachedRuntimeCapabilities: runtime.exposure,
      attachedRuntimeLocked: runtime.lockedSelections,
      nodeCapabilities: {
        ...emptyCapabilities,
        models: ['gpt-5.6'],
        reasoningModes: ['high'],
      },
      pinnedDefaults: { model: 'gpt-5.6', reasoningMode: 'high', sandboxMode: null },
      resolutionDigest: 'sha256:resolved-profile',
    };
    render(<SessionProfileInspector profile={profile} />);

    expect(screen.getByText('reviewer · revision 2')).toBeVisible();
    expect(screen.getByText('sha256:resolved-profile')).toBeVisible();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    expect(screen.queryByText('Initial prompt')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Expand Attached runtime' }));
    expect(screen.getByText('maintain-slice-plan')).toBeVisible();
  });
});
