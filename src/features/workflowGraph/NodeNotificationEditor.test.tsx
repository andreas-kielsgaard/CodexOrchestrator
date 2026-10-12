import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { NodeNotificationEditor } from './NodeNotificationEditor';

it('allows the host to save a node completion sound preference', () => {
  const onChange = vi.fn();
  render(<NodeNotificationEditor value={{ turnCompletionSound: true }} onChange={onChange} />);
  fireEvent.click(screen.getByLabelText('Sound when a turn finishes'));
  expect(onChange).toHaveBeenCalledWith({ turnCompletionSound: false });
});
