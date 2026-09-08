import { fireEvent, render, screen } from '@testing-library/react';
import { PerMessageRuntimeControls } from './PerMessageRuntimeControls';

describe('PerMessageRuntimeControls', () => {
  it('emits controlled options for one message without owning Session state', () => {
    const onChange = vi.fn();
    render(
      <PerMessageRuntimeControls
        value={{ model: null, reasoningMode: null }}
        models={[
          { value: 'gpt-5.6', label: 'GPT-5.6' },
          { value: 'gpt-5.4', label: 'GPT-5.4', disabled: true },
        ]}
        reasoningModes={[{ value: 'high', label: 'High' }]}
        defaultModelLabel="Session model · GPT-5.5"
        defaultReasoningLabel="Session reasoning · medium"
        onChange={onChange}
      />,
    );

    expect(screen.getByText('These choices apply only to the next user message.')).toBeVisible();
    expect(screen.getByRole('option', { name: 'GPT-5.4' })).toBeDisabled();

    fireEvent.change(screen.getByLabelText('Model'), { target: { value: 'gpt-5.6' } });
    expect(onChange).toHaveBeenCalledWith({ model: 'gpt-5.6', reasoningMode: null });

    fireEvent.change(screen.getByLabelText('Reasoning'), { target: { value: 'high' } });
    expect(onChange).toHaveBeenCalledWith({ model: null, reasoningMode: 'high' });
  });
});
