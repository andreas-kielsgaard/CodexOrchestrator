import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import {
  recordedHarnessInspectorSessionId,
  recordedHarnessInspectorSource,
} from '../../dev/conversationHarnesses/recordedHarnessInspectorSource';
import { ConversationHarnessInspector } from './ConversationHarnessInspector';
import { HarnessAwareAgentSessionPane } from './HarnessAwareAgentSessionPane';

describe('HarnessAwareAgentSessionPane', () => {
  it('only offers inspection when a product context supplies a harness source', () => {
    const { rerender } = render(
      <HarnessAwareAgentSessionPane sessionId="session-without-harness">
        <div>Neutral conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(screen.queryByRole('button', { name: 'Inspect harness' })).toBeNull();

    rerender(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={recordedHarnessInspectorSource}
      >
        <div>Product conversation</div>
      </HarnessAwareAgentSessionPane>,
    );
    expect(screen.getByRole('button', { name: 'Inspect harness' })).toBeVisible();
  });

  it('replaces the conversation with a truthful read-only inspector and returns clearly', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId={recordedHarnessInspectorSessionId}
        source={recordedHarnessInspectorSource}
      >
        <section aria-label="Product conversation">Conversation body</section>
      </HarnessAwareAgentSessionPane>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Inspect harness' }));

    expect(
      await screen.findByRole('region', { name: 'Conversation Harness inspector' }),
    ).toBeVisible();
    expect(screen.queryByLabelText('Product conversation')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Epic Plan Builder' })).toBeVisible();
    expect(screen.getByText('Profile configuration · Read only')).toBeVisible();
    expect(screen.getByText('Delivery not evidenced')).toBeVisible();
    expect(screen.getByText('Validation unverified')).toBeVisible();
    expect(screen.queryByText('Delivered · immutable')).toBeNull();
    expect(screen.getAllByText('Future invocation · Read only')).toHaveLength(4);
    expect(screen.getByLabelText('Initial context prefix')).toHaveAttribute('readonly');
    expect(screen.getByLabelText('Model')).toBeDisabled();
    expect(screen.getByLabelText('Sandbox')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Apply to future invocation' })).toBeDisabled();
    expect(screen.getByText('Configuration apply')).toBeVisible();
    expect(screen.getAllByText('unsupported')).not.toHaveLength(0);

    fireEvent.click(screen.getByRole('button', { name: 'Back to conversation' }));
    await waitFor(() => expect(screen.getByLabelText('Product conversation')).toBeVisible());
    expect(screen.queryByRole('region', { name: 'Conversation Harness inspector' })).toBeNull();
  });

  it('keeps an unavailable source explicit and still provides a return', async () => {
    render(
      <HarnessAwareAgentSessionPane
        sessionId="unknown-session"
        source={recordedHarnessInspectorSource}
      >
        <div>Conversation body</div>
      </HarnessAwareAgentSessionPane>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Inspect harness' }));
    expect(await screen.findByText('Harness configuration unavailable')).toBeVisible();
    expect(
      screen.getByText('This recorded Agent Session has no product harness configuration.'),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Back to conversation' })).toBeVisible();
  });

  it('presents invalid validation separately from unverified validation', async () => {
    const read = await recordedHarnessInspectorSource.load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    expect(read.kind).toBe('available');
    if (read.kind !== 'available') return;

    render(
      <ConversationHarnessInspector
        read={{
          kind: 'available',
          snapshot: {
            ...read.snapshot,
            validation: { ...read.snapshot.validation, status: 'invalid' },
            promptContext: {
              ...read.snapshot.promptContext,
              delivery: {
                ...read.snapshot.promptContext.delivery,
                status: 'delivered',
                detail: 'A durable delivery record exists.',
              },
              state: {
                scope: 'application_owned',
                editability: 'unsupported',
                reason: 'Application-owned context.',
              },
            },
          },
        }}
        onBack={() => undefined}
      />,
    );

    expect(screen.getByText('Validation invalid')).toBeVisible();
    expect(screen.queryByText('Validation unverified')).toBeNull();
    expect(screen.getByText('Delivery evidenced')).toBeVisible();
    expect(screen.getAllByText('Application owned · Unsupported')).toHaveLength(2);
  });
});
