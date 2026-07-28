import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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

  it('presents the management hierarchy without development proof language', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <section aria-label="Product conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );

    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));

    expect(await screen.findByRole('region', { name: 'Harness Management' })).toBeVisible();
    expect(screen.queryByLabelText('Product conversation')).toBeNull();
    expect(screen.getByText('Harness')).toBeVisible();
    expect(screen.getAllByText('Epic Plan Builder').length).toBeGreaterThan(0);
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Skill policy' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Tool policy' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Version history' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Agent Session updates' })).toBeVisible();
    expect(
      screen.getAllByLabelText(
        /Antoni Gaudi|Zaha Hadid|Maya Lin|I M Pei|Frank Lloyd Wright|Eero Saarinen|Lina Bo Bardi|Buckminster Fuller|Jane Jacobs|Christopher Wren/,
      ).length,
    ).toBeGreaterThan(0);
    expect(
      screen.queryByText(/validation|provenance|delivery not evidenced|future invocation/i),
    ).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit' })).toBeEnabled();
    expect(screen.getByLabelText('Sandbox')).toBeEnabled();

    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await waitFor(() => expect(screen.getByLabelText('Product conversation')).toBeVisible());
    expect(screen.queryByRole('region', { name: 'Harness Management' })).toBeNull();
  });

  it('keeps edited Markdown in the Session-owned working copy across navigation and remount', async () => {
    const source = createRecordedHarnessManagementSource();
    const first = render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Planning view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Prompt prefix'), {
      target: { value: '# Revised prefix\n\nKeep this durable working copy.' },
    });
    expect((await screen.findAllByText('Uncommitted changes')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await screen.findByLabelText('Planning view conversation');
    first.unmount();

    render(
      <HarnessAwareAgentSessionPane sessionId={recordedHarnessInspectorSessionId} source={source}>
        <section aria-label="Agent Sessions view conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));

    expect(await screen.findByRole('heading', { name: 'Revised prefix' })).toBeVisible();
    expect(screen.getAllByText('Uncommitted changes').length).toBeGreaterThan(0);
  });

  it('records commit, push, and next-prompt update choices as separate preview transitions', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={createRecordedHarnessManagementSource()}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'Manage harness' }));
    fireEvent.change(await screen.findByLabelText('Harness name'), {
      target: { value: 'Epic Plan Builder Plus' },
    });
    expect((await screen.findAllByText('Uncommitted changes')).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: 'Commit version' }));
    expect((await screen.findAllByText('Committed, not active')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: 'Push version' }));
    await waitFor(() => expect(screen.getAllByText('Up to date').length).toBeGreaterThan(0));
    fireEvent.click(screen.getByRole('button', { name: 'Apply updated harness' }));
    expect(
      await screen.findByText('The update choice is recorded for this session in the preview.'),
    ).toBeVisible();
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
    expect(screen.getByRole('button', { name: 'Back to conversation' })).toBeVisible();
  });

  it('does not offer a control when the source reports an unbound Session', async () => {
    const source = createRecordedHarnessManagementSource();
    render(
      <HarnessAwareAgentSessionPane sessionId="unknown-session" source={source}>
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );

    expect(await screen.findByText('Conversation body')).toBeVisible();
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Manage harness' })).toBeNull(),
    );
  });
});
