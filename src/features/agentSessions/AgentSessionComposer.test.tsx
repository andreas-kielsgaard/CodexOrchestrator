import { useState } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AgentSessionComposer } from './AgentSessionComposer';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

const capabilities: AgentSessionQuickFeatures = {
  profileRef: 'test',
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
  active = false,
  contextKey = 'session-1',
}: {
  load: () => Promise<AgentSessionQuickFeatures>;
  send: ReturnType<typeof vi.fn>;
  active?: boolean;
  contextKey?: string;
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
      steeringAvailable
      canceling={false}
      showWorkingDirectory={false}
      keyboardHint="tooltip"
      onWorkingDirectoryChange={() => {}}
      onCancel={() => {}}
      onSend={() => send(draft, selection)}
      quickFeatures={{ contextKey, load, selection, setSelection }}
    />
  );
}

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
  expect(screen.getByRole('option', { name: /Use Session default/ })).toHaveAttribute(
    'aria-disabled',
    'true',
  );
  await user.keyboard('{Escape}{Escape}');
  await user.clear(input);
  await user.type(input, 'Message{Enter}');
  expect(send).toHaveBeenCalledWith('Message', { model: 'model-b', reasoningMode: 'medium' });
  await user.clear(input);
  await user.type(input, '/model');
  await screen.findByRole('option', { name: /Model/ });
  await user.keyboard('{Enter}');
  await user.type(input, 'default{Enter}');
  await user.type(input, 'Inherited{Enter}');
  expect(send).toHaveBeenLastCalledWith('Inherited', { model: null, reasoningMode: 'deep' });
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
