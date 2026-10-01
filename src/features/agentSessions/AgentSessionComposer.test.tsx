import { useState } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AgentSessionComposer } from './AgentSessionComposer';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

const capabilities: AgentSessionQuickFeatures = {
  configuration: { provider: 'codex', configurationId: 'test' },
  defaults: { model: 'model-a', reasoningMode: 'deep' },
  models: [
    {
      id: 'model-a',
      label: 'Model A',
      description: 'First model',
      defaultReasoningMode: 'deep',
      reasoningModes: [
        { id: 'light', description: 'Quick responses' },
        { id: 'deep', description: 'Careful reasoning' },
      ],
    },
    {
      id: 'model-b',
      label: 'Model B',
      description: 'Second model',
      defaultReasoningMode: 'medium',
      reasoningModes: [{ id: 'medium', description: 'Balanced' }],
    },
  ],
  skills: [
    { id: 'review-id', name: 'review', description: 'Review changes', invocationText: '$review' },
  ],
  limitations: [],
};

function Harness({
  load,
  send,
  steer,
  active = false,
  steeringAvailable = true,
  contextKey = 'session-1',
  prefetched = false,
  compact,
  cancel,
}: {
  load: () => Promise<AgentSessionQuickFeatures>;
  send: ReturnType<typeof vi.fn>;
  steer?: ReturnType<typeof vi.fn>;
  active?: boolean;
  steeringAvailable?: boolean;
  contextKey?: string;
  prefetched?: boolean;
  compact?: () => Promise<void>;
  cancel?: () => void;
}) {
  const [draft, setDraft] = useState('');
  const [selection, setSelection] = useState<PerMessageRuntimeSelection>({
    model: null,
    reasoningMode: null,
  });
  return (
    <AgentSessionComposer
      draft={draft}
      onDraftChange={setDraft}
      workingDirectory=""
      isNewSession={false}
      sending={false}
      active={active}
      steeringAvailable={steeringAvailable}
      canceling={false}
      showWorkingDirectory={false}
      keyboardHint="tooltip"
      onWorkingDirectoryChange={() => {}}
      onCancel={cancel ?? (() => {})}
      onSend={(attachments) =>
        attachments.length ? send(draft, selection, attachments) : send(draft, selection)
      }
      onSteer={
        steer
          ? (attachments) => (attachments.length ? steer(draft, attachments) : steer(draft))
          : undefined
      }
      quickFeatures={{
        contextKey,
        load,
        selection,
        setSelection,
        compact,
        ...(prefetched ? { catalogue: capabilities } : {}),
      }}
    />
  );
}

it('offers /compact as a command and queues it without sending a message', async () => {
  const compact = vi.fn(async () => undefined);
  const send = vi.fn();
  render(
    <Harness load={vi.fn(async () => capabilities)} send={send} prefetched compact={compact} />,
  );
  const input = screen.getByRole('textbox', { name: 'Message' });

  await userEvent.setup().type(input, '/compact{Enter}');

  expect(compact).toHaveBeenCalledTimes(1);
  expect(send).not.toHaveBeenCalled();
  expect(input).toHaveValue('');
  expect(screen.getByRole('status')).toHaveTextContent('Compaction added to the queue');
});

it('filters a prefetched catalogue without reloading or clearing results while typing', async () => {
  const load = vi.fn(async () => capabilities);
  const user = userEvent.setup();
  render(<Harness load={load} send={vi.fn()} prefetched />);
  const input = screen.getByRole('textbox', { name: 'Message' });
  await user.type(input, '/');
  expect(screen.getByRole('option', { name: /Skills/ })).toBeVisible();
  await user.type(input, 'sk');
  expect(screen.getByRole('option', { name: /Skills/ })).toBeVisible();
  expect(load).not.toHaveBeenCalled();
});

function setup(active = false) {
  const load = vi.fn(async () => capabilities);
  const send = vi.fn();
  const result = render(<Harness load={load} send={send} active={active} />);
  return {
    ...result,
    load,
    send,
    user: userEvent.setup(),
    input: screen.getByRole('textbox', { name: 'Message' }),
  };
}

it('replaces the command with /, selects a reasoning level, and sends only the later message', async () => {
  const { input, user, send, load } = setup();
  await user.type(input, '/reasoning');
  await screen.findByRole('option', { name: /Reasoning/ });
  await user.keyboard('{Enter}');
  expect(input).toHaveValue('/');
  expect(screen.getByRole('listbox', { name: 'Reasoning' })).toBeVisible();
  await user.type(input, 'light{Enter}');
  expect(input).toHaveValue('');
  expect(send).not.toHaveBeenCalled();
  expect(load).toHaveBeenCalledTimes(1);
  await user.type(input, 'Explain this{Enter}');
  expect(send).toHaveBeenCalledWith('Explain this', { model: null, reasoningMode: 'light' });
});

it('uses model-specific reasoning and replaces an incompatible inherited effort', async () => {
  const { input, user, send } = setup();
  await user.type(input, '/model');
  await screen.findByRole('option', { name: /Model/ });
  await user.keyboard('{Enter}');
  await user.type(input, 'model-b{Enter}');
  await user.type(input, '/reasoning');
  await screen.findByRole('option', { name: /Reasoning/ });
  await user.keyboard('{Enter}');
  expect(screen.queryByRole('option', { name: /light/ })).not.toBeInTheDocument();
  expect(screen.getByRole('option', { name: /medium/ })).toBeVisible();
  expect(screen.queryByRole('option', { name: /Use Session default/ })).not.toBeInTheDocument();
  await user.keyboard('{Escape}{Escape}');
  await user.clear(input);
  await user.type(input, 'Message{Enter}');
  expect(send).toHaveBeenCalledWith('Message', { model: 'model-b', reasoningMode: 'medium' });
  await user.clear(input);
  await user.type(input, '/model');
  await screen.findByRole('option', { name: /Model/ });
  await user.keyboard('{Enter}');
  await user.type(input, 'model-a{Enter}');
  await user.type(input, 'Inherited{Enter}');
  expect(send).toHaveBeenLastCalledWith('Inherited', { model: 'model-a', reasoningMode: 'deep' });
});

it('inserts a skill using provider-owned syntax without submitting it', async () => {
  const { input, user, send } = setup();
  await user.type(input, '/skills');
  await screen.findByRole('option', { name: /Skills/ });
  await user.keyboard('{Enter}');
  await user.type(input, 'review{Enter}');
  expect(input).toHaveValue('$review ');
  expect(send).not.toHaveBeenCalled();
  await user.type(input, 'these changes{Enter}');
  expect(send).toHaveBeenCalledWith('$review these changes', { model: null, reasoningMode: null });
});

it('finds skills from the root and supports mouse selection without losing focus', async () => {
  const { input, user, send } = setup();
  await user.type(input, '/review');
  await user.click(await screen.findByRole('option', { name: /review/ }));
  expect(input).toHaveValue('$review ');
  expect(input).toHaveFocus();
  expect(send).not.toHaveBeenCalled();
});

it('keeps unknown commands local until Escape, while ordinary paths and prose still send', async () => {
  const { input, user, send } = setup();
  await user.type(input, '/unknown');
  await screen.findByText('No matching choices');
  await user.keyboard('{Enter}');
  await user.click(screen.getByRole('button', { name: 'Send' }));
  expect(send).not.toHaveBeenCalled();
  await user.keyboard('{Escape}{Enter}');
  expect(send).toHaveBeenLastCalledWith('/unknown', expect.anything());
  await user.clear(input);
  await user.type(input, '/tmp/example{Enter}');
  expect(send).toHaveBeenLastCalledWith('/tmp/example', expect.anything());
});

it('handles arrows, Tab, Backspace, Shift+Enter, and IME composition separately from sending', async () => {
  const { input, user, send } = setup();
  await user.type(input, '/');
  await screen.findByRole('option', { name: /Reasoning/ });
  await user.keyboard('{ArrowDown}{Tab}');
  expect(screen.getByRole('listbox', { name: 'Reasoning' })).toBeVisible();
  await user.keyboard('{Backspace}');
  expect(input).toHaveValue('/');
  fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
  expect(input).toHaveValue('/');
  expect(send).not.toHaveBeenCalled();
  await user.clear(input);
  await user.type(input, 'First{Shift>}{Enter}{/Shift}Second');
  expect(input).toHaveValue('First\nSecond');
  fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
  expect(send).not.toHaveBeenCalled();
});

it('allows draft model changes during a turn and still offers skills', async () => {
  const { input, user, send } = setup(true);
  await user.type(input, '/model');
  expect(await screen.findByRole('option', { name: /Model/ })).toHaveAttribute(
    'aria-disabled',
    'false',
  );
  await user.keyboard('{Enter}');
  expect(input).toHaveValue('/');
  expect(send).not.toHaveBeenCalled();
  await user.clear(input);
  await user.type(input, '/review');
  await screen.findByRole('option', { name: /review/ });
  await user.keyboard('{Enter}');
  expect(input).toHaveValue('$review ');
});

it('uses Ctrl+Enter for steering when available', async () => {
  const send = vi.fn();
  const steer = vi.fn();
  render(<Harness load={vi.fn(async () => capabilities)} send={send} steer={steer} active />);
  const input = screen.getByRole('textbox', { name: 'Message' });
  await userEvent.setup().type(input, 'Correct this');
  fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
  expect(steer).toHaveBeenCalledWith('Correct this');
  expect(send).not.toHaveBeenCalled();
});

it('does not silently queue Ctrl+Enter when steering is unavailable', async () => {
  const send = vi.fn();
  const steer = vi.fn();
  render(
    <Harness
      load={vi.fn(async () => capabilities)}
      send={send}
      steer={steer}
      active
      steeringAvailable={false}
    />,
  );
  const input = screen.getByRole('textbox', { name: 'Message' });
  await userEvent.setup().type(input, 'Queue this');
  fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
  expect(send).not.toHaveBeenCalled();
  expect(steer).not.toHaveBeenCalled();
  expect(input).toHaveValue('Queue this');
  expect(screen.getByText(/Your message has not been queued/)).toBeInTheDocument();
});

it('retains the draft on discovery failure and retries without sending', async () => {
  const load = vi
    .fn()
    .mockRejectedValueOnce(new Error('Provider offline'))
    .mockResolvedValue(capabilities);
  const send = vi.fn();
  render(<Harness load={load} send={send} />);
  const user = userEvent.setup();
  const input = screen.getByRole('textbox', { name: 'Message' });
  await user.type(input, '/reasoning');
  await screen.findByText('Provider offline');
  await user.keyboard('{Enter}');
  expect(send).not.toHaveBeenCalled();
  expect(input).toHaveValue('/reasoning');
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  await screen.findByRole('option', { name: /Reasoning/ });
  expect(load).toHaveBeenCalledTimes(2);
});

it('ignores a late capability response from a different session context', async () => {
  let resolveFirst!: (value: AgentSessionQuickFeatures) => void;
  const first = new Promise<AgentSessionQuickFeatures>((resolve) => {
    resolveFirst = resolve;
  });
  const load = vi
    .fn()
    .mockReturnValueOnce(first)
    .mockResolvedValue({ ...capabilities, skills: [] });
  const send = vi.fn();
  const { rerender } = render(<Harness load={load} send={send} />);
  await userEvent.setup().type(screen.getByRole('textbox', { name: 'Message' }), '/');
  await waitFor(() => expect(load).toHaveBeenCalledTimes(1));
  rerender(<Harness load={load} send={send} contextKey="session-2" />);
  await screen.findByRole('option', { name: /Reasoning/ });
  await act(async () => resolveFirst(capabilities));
  expect(screen.queryByRole('option', { name: /review/ })).not.toBeInTheDocument();
  expect(send).not.toHaveBeenCalled();
});

it('attaches an image without text, previews it, and sends portable content', async () => {
  const send = vi.fn();
  render(<Harness load={async () => capabilities} send={send} />);
  const image = new File([new Uint8Array([1, 2, 3])], 'diagram.png', { type: 'image/png' });
  Object.defineProperty(image, 'arrayBuffer', {
    value: async () => new Uint8Array([1, 2, 3]).buffer,
  });
  await userEvent.setup().upload(screen.getByLabelText('Image files'), image);
  expect(await screen.findByRole('img', { name: 'diagram.png' })).toBeVisible();
  await userEvent.setup().click(screen.getByRole('button', { name: 'Send' }));
  expect(send).toHaveBeenCalledTimes(1);
  expect(send.mock.calls[0][2]).toEqual([
    expect.objectContaining({ name: 'diagram.png', mediaType: 'image/png', dataBase64: 'AQID' }),
  ]);
});

it('shows one compact attachment preview and reveals the collection and removal actions on hover', async () => {
  const user = userEvent.setup();
  render(<Harness load={async () => capabilities} send={vi.fn()} />);
  const files = ['first.png', 'second.png'].map((name, index) => {
    const bytes = new Uint8Array([index + 1]);
    const file = new File([bytes], name, { type: 'image/png' });
    Object.defineProperty(file, 'arrayBuffer', { value: async () => bytes.buffer });
    return file;
  });
  await user.upload(screen.getByLabelText('Image files'), files);
  await screen.findByRole('img', { name: 'first.png' });
  const preview = screen.getByRole('button', { name: 'Attached images' });
  expect(preview).toHaveTextContent('+1');
  expect(screen.queryByRole('img', { name: 'second.png' })).not.toBeInTheDocument();
  expect(preview.closest('.composer-input-row')).toBeInTheDocument();
  await user.hover(preview);
  expect(screen.getByRole('img', { name: 'second.png' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove second.png' }));
  expect(preview).not.toHaveTextContent('+1');
  expect(screen.queryByRole('img', { name: 'second.png' })).not.toBeInTheDocument();
});

it('keeps active-turn cancellation reachable by keyboard when the Queue button is disabled', async () => {
  const user = userEvent.setup();
  const cancel = vi.fn();
  render(<Harness load={async () => capabilities} send={vi.fn()} active cancel={cancel} />);
  expect(screen.getByRole('button', { name: 'Queue' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument();
  await user.tab(); // Message
  await user.tab(); // Attach images
  await user.tab(); // Turn actions, even with an empty draft
  expect(screen.getByRole('button', { name: 'Cancel' })).toBeVisible();
  await user.tab();
  expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
  await user.keyboard('{Enter}');
  expect(cancel).toHaveBeenCalledTimes(1);
});

it('pastes clipboard images without replacing ordinary text paste', async () => {
  const send = vi.fn();
  render(<Harness load={async () => capabilities} send={send} />);
  const input = screen.getByRole('textbox', { name: 'Message' });
  const image = new File([new Uint8Array([4, 5, 6])], '', { type: 'image/png' });
  Object.defineProperty(image, 'arrayBuffer', {
    value: async () => new Uint8Array([4, 5, 6]).buffer,
  });

  const imagePasteAccepted = fireEvent.paste(input, {
    clipboardData: {
      items: [{ kind: 'file', type: 'image/png', getAsFile: () => image }],
      files: [image],
    },
  });

  expect(imagePasteAccepted).toBe(false);
  expect(await screen.findByRole('img', { name: 'Pasted image 1.png' })).toBeVisible();
  const textPasteAccepted = fireEvent.paste(input, {
    clipboardData: { items: [], files: [], getData: () => 'ordinary text' },
  });
  expect(textPasteAccepted).toBe(true);
  await userEvent.setup().click(screen.getByRole('button', { name: 'Send' }));
  expect(send.mock.calls[0][2]).toEqual([
    expect.objectContaining({
      name: 'Pasted image 1.png',
      mediaType: 'image/png',
      dataBase64: 'BAUG',
    }),
  ]);
});
