import { render, screen, fireEvent } from '@testing-library/react';
import { useRef, useState } from 'react';
import { useComposerAutosize } from './useComposerAutosize';

it('grows to two-thirds of the view and shrinks again with the draft', () => {
  const height = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(900);
  let contentHeight = 80;
  const scroll = vi
    .spyOn(HTMLElement.prototype, 'scrollHeight', 'get')
    .mockImplementation(() => contentHeight);
  function Harness() {
    const ref = useRef<HTMLTextAreaElement>(null);
    const [draft, setDraft] = useState('');
    useComposerAutosize(ref, draft);
    return (
      <div className="agent-session-workspace">
        <textarea
          ref={ref}
          aria-label="Draft"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </div>
    );
  }
  render(<Harness />);
  const input = screen.getByLabelText('Draft');
  expect(input).toHaveStyle({ height: '80px' });
  contentHeight = 1100;
  fireEvent.change(input, { target: { value: 'Long message' } });
  expect(input).toHaveStyle({ height: '600px', overflowY: 'auto' });
  contentHeight = 70;
  fireEvent.change(input, { target: { value: '' } });
  expect(input).toHaveStyle({ height: '70px', overflowY: 'hidden' });
  height.mockRestore();
  scroll.mockRestore();
});
