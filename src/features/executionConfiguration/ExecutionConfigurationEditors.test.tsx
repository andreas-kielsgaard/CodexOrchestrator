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
  profileRef: 'native-codex/global',
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

function ControlledCapabilityEditor({ onSave }: { onSave(profile: CapabilityProfileDraft): void }) {
  const [profile, setProfile] = useState<CapabilityProfileDraft>({
    capabilityProfileId: '',
    name: '',
    revision: null,
    allowedCapabilities: emptyCapabilities,
  });
  return (
    <CapabilityProfileEditor
      profile={profile}
      runtime={runtime}
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

    await user.type(screen.getByRole('textbox', { name: 'Capability profile ID' }), 'reviewer');
    await user.type(screen.getByRole('textbox', { name: 'Capability profile name' }), 'Reviewer');
    await user.click(screen.getByRole('checkbox', { name: 'GPT 5.6' }));
    await user.click(screen.getByRole('checkbox', { name: 'Message Session' }));
    await user.click(screen.getByRole('button', { name: 'Create profile' }));

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        capabilityProfileId: 'reviewer',
        name: 'Reviewer',
        allowedCapabilities: expect.objectContaining({
          models: ['gpt-5.6'],
          mcpTools: { orchestrator: ['session-message'] },
        }),
      }),
    );
  });

  it('shows unavailable runtime catalogs without inventing editable options', () => {
    const reason = 'Skill discovery is controlled by the selected Codex profile.';
    render(
      <CapabilityProfileEditor
        profile={{
          capabilityProfileId: 'reviewer',
          name: 'Reviewer',
          revision: 2,
          allowedCapabilities: emptyCapabilities,
        }}
        runtime={{
          ...runtime,
          catalogs: {
            ...runtime.catalogs,
            skills: {
              availability: 'unavailable',
              options: [],
              reason,
            },
          },
        }}
        onChange={() => undefined}
      />,
    );

    expect(screen.getByText(reason)).toBeVisible();
    expect(screen.getByRole('group', { name: /Skills/ })).toBeDisabled();
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
    expect(screen.getByRole('combobox', { name: 'Default sandbox' })).toBeDisabled();

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
      runtimeProfileRef: runtime.profileRef,
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
