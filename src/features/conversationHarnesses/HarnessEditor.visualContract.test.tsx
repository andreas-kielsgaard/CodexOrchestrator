import { fireEvent, render, screen, within } from '@testing-library/react';
import { vi } from 'vitest';
import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from '../../dev/conversationHarnesses/recordedHarnessInspectorSource';
import { HarnessEditor } from './HarnessEditor';

describe('HarnessEditor visual contract', () => {
  it('keeps the reviewed management hierarchy instead of presenting a generic field form', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    const { container } = render(
      <HarnessEditor read={read} onBack={vi.fn()} onCommand={vi.fn()} />,
    );

    expect(container.querySelector('.harness-editor')).toHaveAttribute(
      'data-harness-editor-layout',
      'reviewed-management',
    );
    expect(screen.getByLabelText('Harness Management controls')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Back to conversation' })).toBeVisible();
    expect(screen.getByLabelText('Viewed harness version')).toHaveValue('version:3');
    expect(screen.getByRole('button', { name: 'Edit Harness' })).toBeVisible();

    const sectionTitles = [
      ...container.querySelectorAll('.harness-management__card > header h2'),
    ].map((heading) => heading.textContent);
    expect(sectionTitles).toEqual([
      'Harness details',
      'Prompt prefix',
      'Skills',
      'Tools',
      'Models and reasoning',
      'Sandbox and authority',
      'Application hooks',
      'Version history',
    ]);
    expect(screen.getAllByRole('button', { name: /^Collapse / })).toHaveLength(8);

    expect(screen.getByRole('button', { name: /Permitted name pool/ })).toBeVisible();
    expect(screen.queryByLabelText('Machine key')).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit Agent color and shape' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit skills' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit tools' })).toBeVisible();
    expect(screen.getAllByRole('button', { name: /Always applicable/ })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: /Initial ingestion only/ })).toHaveLength(2);
    expect(screen.getByLabelText('Harness preferred model')).toBeDisabled();
    expect(screen.getByLabelText('Harness preferred reasoning')).toBeDisabled();
    expect(screen.queryByRole('slider')).toBeNull();
    expect(screen.queryByLabelText(/Harness allows/)).toBeNull();
    expect(screen.queryByText(/delegated|shared policy|version specific/i)).toBeNull();

    expect(screen.queryByLabelText('Harness skills')).toBeNull();
    expect(screen.queryByLabelText('Harness tools')).toBeNull();
    expect(screen.queryByLabelText('Allowed models')).toBeNull();
    expect(container.querySelector('.harness-definition-selector')).toBeNull();

    const promptToggle = screen.getByRole('button', { name: 'Collapse Prompt prefix' });
    fireEvent.click(promptToggle);
    expect(screen.getByRole('button', { name: 'Expand Prompt prefix' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.getByText(/You are the Epic Plan Builder/)).not.toBeVisible();
  });

  it('keeps catalog editing and item inspection in focused dialogs', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    render(<HarnessEditor read={read} onBack={vi.fn()} onCommand={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /Permitted name pool/ }));
    const names = screen.getByRole('dialog', { name: 'Permitted name pool' });
    expect(within(names).getByLabelText('Search product names')).toBeVisible();
    expect(within(names).getByLabelText('Antoni Gaudi permitted')).toBeChecked();
    expect(within(names).getByLabelText('Grace Hopper permitted')).not.toBeChecked();
    fireEvent.click(within(names).getByRole('button', { name: 'Close permitted name pool' }));

    fireEvent.click(screen.getByRole('button', { name: 'Edit Agent color and shape' }));
    const identity = screen.getByRole('dialog', { name: 'Current Agent identity' });
    expect(within(identity).getByLabelText('Identity color')).toHaveValue('#39745a');
    expect(within(identity).getByRole('radio', { name: 'Circle' })).toBeChecked();
    expect(within(identity).getByRole('radio', { name: 'Square' })).not.toBeChecked();
    expect(within(identity).getByRole('radio', { name: 'Hexagon' })).not.toBeChecked();
    fireEvent.click(within(identity).getByRole('button', { name: 'Close Current Agent identity' }));

    fireEvent.click(screen.getByRole('button', { name: 'Edit skills' }));
    const skills = screen.getByRole('dialog', { name: 'Edit skills' });
    expect(within(skills).getByLabelText('Search all skills')).toBeVisible();
    expect(within(skills).getByRole('heading', { name: 'Selected skills' })).toBeVisible();
    expect(within(skills).getByRole('heading', { name: 'Skill catalog' })).toBeVisible();
  });

  it('maps the shared Session identity picker back to the legacy management command', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    const onCommand = vi.fn();
    render(<HarnessEditor read={read} onBack={vi.fn()} onCommand={onCommand} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit Agent identity for Avery' }));
    const identity = screen.getByRole('dialog', { name: 'Current Agent identity' });
    fireEvent.change(within(identity).getByLabelText('Identity display name'), {
      target: { value: 'Avery Stone' },
    });
    fireEvent.change(within(identity).getByLabelText('Identity color'), {
      target: { value: '#2456aa' },
    });
    fireEvent.click(within(identity).getByRole('radio', { name: 'Square' }));
    fireEvent.click(within(identity).getByRole('button', { name: 'Apply to this Session' }));

    expect(onCommand).toHaveBeenCalledWith({
      kind: 'update_session_identity',
      name: 'Avery Stone',
      visualIdentity: {
        token: 'drafting_compass',
        accent: '#2456aa',
        shape: 'square',
      },
    });
  });

  it('displays reusable Identity definitions while saving only their IDs in the Harness policy', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    if (read.kind !== 'available') throw new Error('Expected the recorded Harness to load.');
    const base = read.snapshot.versionControl.versions.find(({ revision }) => revision === 3);
    if (!base) throw new Error('Expected Harness version 3.');
    const configuration = {
      ...base.configuration,
      identity: {
        ...base.configuration.identity,
        permittedAgentNames: ['identity-avery', 'identity-retired'],
      },
    };
    const identityRead = {
      ...read,
      snapshot: {
        ...read.snapshot,
        catalogs: {
          ...read.snapshot.catalogs,
          identities: {
            source: 'application_identity_catalog' as const,
            items: [
              {
                id: 'identity-avery',
                displayName: 'Avery',
                color: '#4f46e5',
                shape: 'hexagon' as const,
              },
              {
                id: 'identity-grace',
                displayName: 'Grace Hopper',
                color: '#39745a',
                shape: 'circle' as const,
              },
            ],
            reason: 'Application Identity catalog.',
          },
        },
        workingCopy: {
          baseRevision: 3,
          draftRevision: 4,
          dirty: true,
          configuration,
        },
      },
    };
    const onCommand = vi.fn();
    render(<HarnessEditor read={identityRead} onBack={vi.fn()} onCommand={onCommand} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit Harness' }));
    fireEvent.click(screen.getByRole('button', { name: /Permitted identities/ }));
    const dialog = screen.getByRole('dialog', { name: 'Permitted identities' });

    expect(within(dialog).getByLabelText('Avery')).toBeVisible();
    expect(within(dialog).getByLabelText('Grace Hopper')).toBeVisible();
    expect(within(dialog).getByLabelText('Avery permitted')).toBeChecked();
    expect(within(dialog).getByLabelText('Grace Hopper permitted')).not.toBeChecked();
    expect(within(dialog).getByText('Unavailable definition · identity-retired')).toBeVisible();
    fireEvent.click(within(dialog).getByLabelText('Grace Hopper permitted'));

    expect(onCommand).toHaveBeenLastCalledWith({
      kind: 'save_working_copy',
      configuration: expect.objectContaining({
        identity: expect.objectContaining({
          permittedAgentNames: ['identity-avery', 'identity-retired', 'identity-grace'],
        }),
      }),
    });

    fireEvent.change(within(dialog).getByLabelText('Identity pool source'), {
      target: { value: 'full_catalog' },
    });
    expect(onCommand).toHaveBeenLastCalledWith({
      kind: 'save_working_copy',
      configuration: expect.objectContaining({
        identity: expect.objectContaining({ permittedAgentNames: null }),
      }),
    });
  });

  it('keeps a fresh empty Identity catalog unrestricted', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    if (read.kind !== 'available') throw new Error('Expected the recorded Harness to load.');
    const base = read.snapshot.versionControl.versions.find(({ revision }) => revision === 3);
    if (!base) throw new Error('Expected Harness version 3.');
    const emptyCatalogRead = {
      ...read,
      snapshot: {
        ...read.snapshot,
        catalogs: {
          ...read.snapshot.catalogs,
          identities: {
            source: 'application_identity_catalog' as const,
            items: [],
            reason: 'Application Identity catalog.',
          },
        },
        workingCopy: {
          baseRevision: 3,
          draftRevision: 4,
          dirty: true,
          configuration: {
            ...base.configuration,
            identity: { ...base.configuration.identity, permittedAgentNames: null },
          },
        },
      },
    };
    render(<HarnessEditor read={emptyCatalogRead} onBack={vi.fn()} onCommand={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit Harness' }));
    fireEvent.click(screen.getByRole('button', { name: /Permitted identities/ }));
    const dialog = screen.getByRole('dialog', { name: 'Permitted identities' });

    expect(within(dialog).getByLabelText('Identity pool source')).toBeDisabled();
    expect(
      within(dialog).getByText(
        'No reusable identities exist yet. This Harness remains unrestricted.',
      ),
    ).toBeVisible();
  });

  it('separates reusable Harness editing from in-memory Session customization', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    if (read.kind !== 'available') throw new Error('Expected the recorded Harness to load.');
    const onCommand = vi.fn();
    const { rerender } = render(
      <HarnessEditor read={read} onBack={vi.fn()} onCommand={onCommand} />,
    );

    expect(screen.getByRole('button', { name: 'Edit Harness' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Customize this Session' }));
    expect(onCommand).toHaveBeenCalledWith({ kind: 'start_session_edit', baseRevision: 3 });

    const base = read.snapshot.versionControl.versions.find(({ revision }) => revision === 3);
    if (!base) throw new Error('Expected Harness version 3.');
    const sessionRead = {
      ...read,
      snapshot: {
        ...read.snapshot,
        sessionWorkingCopy: {
          baseRevision: 3,
          dirty: true as const,
          configuration: base.configuration,
        },
      },
    };
    rerender(<HarnessEditor read={sessionRead} onBack={vi.fn()} onCommand={onCommand} />);

    expect(screen.getByLabelText('Viewed harness version')).toHaveValue('session-draft');
    expect(screen.getByText('Session draft · in memory · based on v3')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Publish for this Session' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Discard' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Discard' }));
    let confirmation = screen.getByRole('alertdialog', {
      name: 'Discard this Session customization?',
    });
    expect(within(confirmation).getByText(/removes the in-memory Session draft/i)).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Discard Session draft' }));
    expect(onCommand).toHaveBeenCalledWith({ kind: 'discard_session_working_copy' });

    fireEvent.click(screen.getByRole('button', { name: 'Customize this Session' }));
    fireEvent.click(screen.getByRole('button', { name: 'Publish for this Session' }));
    confirmation = screen.getByRole('alertdialog', {
      name: 'Publish this Session customization?',
    });
    expect(
      within(confirmation).getByText(
        /creates a Session-specific immutable Harness version.*updates only this Session/i,
      ),
    ).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Publish for this Session' }));
    expect(onCommand).toHaveBeenCalledWith({
      kind: 'publish_session_override',
      expectedBaseRevision: 3,
    });
  });
});
