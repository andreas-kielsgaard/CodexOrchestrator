import { useRef } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { AddToChatSelection } from './AddToChatSelection';

it('adds selected transcript text with an optional comment, only after confirmation', () => {
  const add = vi.fn();
  function Fixture() {
    const region = useRef<HTMLDivElement>(null);
    return (
      <>
        <div ref={region}>
          <p>Selected passage</p>
        </div>
        <AddToChatSelection region={region} onAdd={add} />
      </>
    );
  }
  render(<Fixture />);
  const text = screen.getByText('Selected passage');
  const selection = {
    isCollapsed: false,
    rangeCount: 1,
    anchorNode: text.firstChild,
    focusNode: text.firstChild,
    toString: () => 'Selected passage\nSecond line',
    getRangeAt: () => ({ getBoundingClientRect: () => ({ left: 10, top: 100 }) }),
    removeAllRanges: vi.fn(),
  };
  vi.spyOn(window, 'getSelection').mockReturnValue(selection as unknown as Selection);
  fireEvent.mouseUp(text);
  fireEvent.click(screen.getByRole('button', { name: 'Add to chat' }));
  expect(add).not.toHaveBeenCalled();
  fireEvent.change(screen.getByRole('textbox', { name: 'Optional quote comment' }), {
    target: { value: 'Explain this' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Add excerpt' }));
  expect(add).toHaveBeenCalledWith('> Selected passage\n> Second line\n\nExplain this');
  expect(selection.removeAllRanges).toHaveBeenCalled();
});
