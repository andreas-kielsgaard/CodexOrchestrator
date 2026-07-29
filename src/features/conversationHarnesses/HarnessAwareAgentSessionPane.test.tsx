import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from '../../dev/conversationHarnesses/recordedHarnessInspectorSource';
import { ConversationHarnessManagement } from './ConversationHarnessInspector';
import { HarnessAwareAgentSessionPane } from './HarnessAwareAgentSessionPane';

describe('HarnessAwareAgentSessionPane', () => {
  it('offers management only after the Session-owned source returns a harness relationship', async () => {
    const source = createRecordedHarnessManagementSource();
    const { rerender } = render(
      <HarnessAwareAgentSessionPane sessionId="session-without-harness">
        <div>Neutral conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(screen.queryByRole('button', { name: 'Manage harness' })).toBeNull();

    rerender(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <div>Product conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(await screen.findByRole('button', { name: 'Manage harness' })).toBeVisible();
  });

  it('defaults to the Session version and presents the corrected management hierarchy', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <section aria-label="Product conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    expect(await screen.findByRole('heading', { name: 'Harness details' })).toBeVisible();
    expect(screen.queryByLabelText('Product conversation')).toBeNull();
    expect(screen.getByLabelText('Viewed harness version')).toHaveValue('version:3');
    expect(screen.getByText('Session version · v3')).toBeVisible();
    expect(screen.getByText('Current pushed · v4')).toBeVisible();
    expect(screen.getByText('Viewed v3 · previously pushed')).toBeVisible();
    expect(screen.queryByText('Viewed version is not pushed')).toBeNull();
    expect(
      screen.getByRole('option', { name: 'v3 · previously pushed · this Session' }),
    ).toBeVisible();
    expect(screen.getByRole('option', { name: 'v4 · current pushed' })).toBeVisible();
    expect(screen.getByText('Epic Plan Builder', { selector: 'strong' })).toBeVisible();
    expect(screen.queryByLabelText('Harness role')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Skills' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Tools' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Models and reasoning' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Application hooks' })).toBeVisible();
    expect(screen.getByText(/Proposed application hook reference:/)).toBeVisible();
    expect(screen.getByText('Proposed', { selector: '.harness-management__badge' })).toBeVisible();
    expect(screen.queryByText(/Connected application hook:/)).toBeNull();
    expect(screen.queryByText('Exposed', { selector: '.harness-management__badge' })).toBeNull();
    expect(screen.getByRole('table')).toHaveAccessibleName('');
    expect(screen.getByRole('heading', { name: 'Version history' })).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'Agent Session updates' })).toBeNull();
    expect(screen.getByRole('slider', { name: 'GPT-5.6 Terra minimum reasoning' })).toBeDisabled();
    expect(
      screen.getAllByLabelText(
        /Antoni Gaudi|Zaha Hadid|Maya Lin|I M Pei|Frank Lloyd Wright|Eero Saarinen|Lina Bo Bardi|Buckminster Fuller|Jane Jacobs|Christopher Wren/,
      ).length,
    ).toBeGreaterThan(0);
    expect(
      screen.queryByText(/validation|provenance|delivery not evidenced|future invocation/i),
    ).toBeNull();

    const always = screen.getAllByRole('button', { name: /Always applicable/ })[0];
    const initial = screen.getAllByRole('button', { name: /Initial ingestion only/ })[0];
    const available = screen.getAllByRole('button', { name: /^Available/ })[0];
    expect(screen.getAllByRole('button', { name: /Always applicable/ })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: /Initial ingestion only/ })).toHaveLength(2);
    expect(screen.queryByRole('button', { name: /Every invocation/ })).toBeNull();
    expect(always).toHaveAttribute('aria-expanded', 'true');
    expect(initial).toHaveAttribute('aria-expanded', 'true');
    expect(available).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await waitFor(() => expect(screen.getByLabelText('Product conversation')).toBeVisible());
  });

  it('inspects and edits the Harness name subset without renaming the existing Session', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    fireEvent.click(screen.getByRole('button', { name: /Permitted name pool/ }));
    const dialog = await screen.findByRole('dialog', { name: 'Permitted name pool' });
    expect(within(dialog).getByLabelText('Eero Saarinen permitted')).toBeChecked();
    expect(within(dialog).getByLabelText('Grace Hopper permitted')).not.toBeChecked();
    expect(within(dialog).getByLabelText('Grace Hopper permitted')).toBeDisabled();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit name pool' }));
    await waitFor(() =>
      expect(within(dialog).getByLabelText('Grace Hopper permitted')).toBeEnabled(),
    );
    fireEvent.click(within(dialog).getByLabelText('Grace Hopper permitted'));
    await waitFor(() =>
      expect(within(dialog).getByLabelText('Grace Hopper permitted')).toBeChecked(),
    );
    fireEvent.click(within(dialog).getByRole('button', { name: 'Close permitted name pool' }));

    expect(screen.getByRole('button', { name: /Harness subset · 11 names/ })).toBeVisible();
    expect(screen.getAllByLabelText('Eero Saarinen, Epic Plan Builder').length).toBeGreaterThan(0);
    expect(
      screen.getByText('Working draft · uncommitted', {
        selector: '.harness-management__badge',
      }),
    ).toBeVisible();
  });

  it('opens full selected-skill details and changes applicability without replacing the catalog flow', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    fireEvent.click(screen.getAllByRole('button', { name: /^Available/ })[0]);
    fireEvent.click(
      await screen.findByRole('button', { name: 'View epic-plan-builder skill details' }),
    );

    const dialog = await screen.findByRole('dialog', { name: 'epic-plan-builder' });
    expect(within(dialog).getByText(/# Epic Plan Builder/)).toBeVisible();
    expect(
      within(dialog).getByText(/application can derive it from the calling session/),
    ).toBeVisible();
    const applicability = within(dialog).getByLabelText('epic-plan-builder details applicability');
    expect(applicability).toBeDisabled();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit skill policy' }));
    await waitFor(() => expect(applicability).toBeEnabled());
    fireEvent.change(applicability, { target: { value: 'always_applicable' } });
    await waitFor(() => expect(applicability).toHaveValue('always_applicable'));
    fireEvent.click(
      within(dialog).getByRole('button', { name: 'Close epic-plan-builder details' }),
    );

    expect(
      screen.getByRole('button', { name: 'View epic-plan-builder skill details' }),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit skills' })).toBeVisible();
  });

  it('keeps one prompt value and caret-safe Markdown/Plain edits across navigation and remount', async () => {
    const source = createRecordedHarnessManagementSource();
    const first = render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Planning view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness name')).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByLabelText('Prompt prefix plain Markdown') as HTMLTextAreaElement;
    fireEvent.change(plain, {
      target: {
        value: '# Revised prefix\n\nKeep this durable working copy.',
      },
    });
    expect(
      await screen.findByText('Working draft · uncommitted', {
        selector: '.harness-management__badge',
      }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await screen.findByLabelText('Planning view conversation');
    first.unmount();

    render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Agent Sessions view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    expect(screen.getByLabelText('Viewed harness version')).toHaveValue('version:3');
    expect(screen.getByText('Working draft has uncommitted changes')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit draft' })).toBeVisible();
    fireEvent.change(screen.getByLabelText('Viewed harness version'), {
      target: { value: 'draft' },
    });
    expect(await screen.findByRole('heading', { name: 'Revised prefix' })).toBeVisible();
  });

  it('searches, adds, categorizes, collapses, and removes skills in the draft dialog', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    fireEvent.click(screen.getByRole('button', { name: 'Edit skills' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edit skills' });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    await waitFor(() => expect(within(dialog).getByLabelText('Available discovery')).toBeEnabled());
    fireEvent.change(within(dialog).getByLabelText('Search all skills'), {
      target: { value: 'sprnr' },
    });
    const result = await within(dialog).findByText('sprint-runner');
    const resultRow = result.closest('.harness-management__catalog-row');
    expect(resultRow).not.toBeNull();
    if (!resultRow) return;
    fireEvent.click(within(resultRow as HTMLElement).getByRole('button', { name: 'Add' }));
    const applicability = await within(dialog).findByLabelText('sprint-runner applicability');
    fireEvent.change(applicability, { target: { value: 'always_applicable' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Close Edit skills' }));

    const always = screen.getAllByRole('button', { name: /Always applicable/ })[0];
    expect(always).toHaveAttribute('aria-expanded', 'true');
    expect(await screen.findByText('sprint-runner')).toBeVisible();
    fireEvent.click(always);
    expect(always).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('sprint-runner')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Edit skills' }));
    const reopened = await screen.findByRole('dialog', { name: 'Edit skills' });
    fireEvent.click(await within(reopened).findByRole('button', { name: 'Remove sprint-runner' }));
    await waitFor(() =>
      expect(within(reopened).queryByLabelText('sprint-runner applicability')).toBeNull(),
    );
  });

  it('edits and removes runtime-owned tools through the searchable tool dialog', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });
    fireEvent.click(screen.getByRole('button', { name: 'Edit tools' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edit tools' });
    const exposure = within(dialog).getByLabelText('request_epic_initiation exposure');
    await waitFor(() => expect(exposure).toBeEnabled());
    expect(within(exposure).getByRole('option', { name: 'Always applicable' })).toBeVisible();
    expect(within(exposure).getByRole('option', { name: 'Initial ingestion only' })).toBeVisible();
    expect(within(exposure).queryByRole('option', { name: 'Every invocation' })).toBeNull();
    fireEvent.change(within(dialog).getByLabelText('Search all tools'), {
      target: { value: 'reqinit' },
    });
    fireEvent.change(exposure, {
      target: { value: 'initial_invocation' },
    });
    fireEvent.click(
      within(dialog).getByRole('button', {
        name: 'Remove submit_epic_plan_proposal',
      }),
    );
    await waitFor(() =>
      expect(within(dialog).queryByLabelText('submit_epic_plan_proposal exposure')).toBeNull(),
    );
    expect(within(dialog).getByText(/schemas remain runtime-owned/i)).toBeVisible();
  });

  it('confirmation-gates Session changes, commit, push, and bulk next-prompt queues', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    await screen.findByRole('heading', { name: 'Harness details' });

    fireEvent.change(screen.getByLabelText('Viewed harness version'), {
      target: { value: 'version:4' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Change this Session to v4' }));
    let confirmation = screen.getByRole('alertdialog', {
      name: 'Change this Session to v4?',
    });
    expect(within(confirmation).getByText(/until its next prompt/i)).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText('Queued for next prompt')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Edit harness' }));
    await waitFor(() => expect(screen.getByLabelText('Harness name')).toBeEnabled());
    fireEvent.change(screen.getByLabelText('Harness name'), {
      target: { value: 'Epic Plan Builder Plus' },
    });
    await screen.findByText('Working draft · uncommitted', {
      selector: '.harness-management__badge',
    });
    fireEvent.click(screen.getByRole('button', { name: 'Commit' }));
    confirmation = screen.getByRole('alertdialog', {
      name: 'Commit this harness version?',
    });
    expect(within(confirmation).getByText(/does not push.*Sessions/i)).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Commit version' }));
    await waitFor(() =>
      expect(screen.getByLabelText('Viewed harness version')).toHaveValue('version:5'),
    );
    expect(screen.getByText('Viewed v5 · committed, not pushed')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Push' }));
    confirmation = screen.getByRole('alertdialog', { name: 'Push harness v5?' });
    expect(
      within(confirmation).getByText(/queues it.*next prompt.*does not contact a remote/i),
    ).toBeVisible();
    expect(within(confirmation).queryByText(/interrupt now/i)).toBeNull();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Push v5' }));
    await waitFor(() => expect(screen.getByText('Queued for next prompt')).toBeVisible());
    expect(screen.getByText('Viewed v5 · current pushed')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Change all to v4' }));
    confirmation = screen.getByRole('alertdialog', {
      name: 'Change all relevant Sessions to v4?',
    });
    expect(within(confirmation).getByText(/each Session.*next prompt/i)).toBeVisible();
    fireEvent.click(within(confirmation).getByRole('button', { name: 'Queue v4 for all' }));
    await waitFor(() => expect(screen.getByText('Queued for next prompt')).toBeVisible());
    expect(screen.queryByRole('button', { name: /interrupt/i })).toBeNull();
  });

  it('keeps unavailable and unbound reads explicit', () => {
    const { rerender } = render(
      <ConversationHarnessManagement
        read={{ kind: 'unavailable', reason: 'The application query failed.' }}
        onBack={() => undefined}
      />,
    );
    expect(screen.getByText('Harness unavailable')).toBeVisible();
    expect(screen.getByText('The application query failed.')).toBeVisible();

    rerender(
      <ConversationHarnessManagement
        read={{ kind: 'unbound', reason: 'The Session has no harness relationship.' }}
        onBack={() => undefined}
      />,
    );
    expect(screen.getByText('No harness assigned')).toBeVisible();
    expect(screen.getByText('The Session has no harness relationship.')).toBeVisible();
  });

  it('does not offer a control when the source reports an unbound Session', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId="unknown-session"
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(await screen.findByText('Conversation body')).toBeVisible();
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Manage harness' })).toBeNull(),
    );
  });
});
