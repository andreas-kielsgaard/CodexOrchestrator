import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { MarkdownEditor } from './MarkdownEditor';

function ControlledEditor({ editable = true }: { readonly editable?: boolean }) {
  const [value, setValue] = useState('# Prompt prefix\n\nUse **careful** guidance.');
  return (
    <MarkdownEditor label="Harness prompt" value={value} editable={editable} onChange={setValue} />
  );
}

describe('MarkdownEditor', () => {
  it('offers formatted Markdown and raw Plain modes over one working value', () => {
    render(<ControlledEditor />);

    expect(screen.getByRole('button', { name: 'Markdown' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByRole('toolbar', { name: 'Markdown formatting' })).toBeVisible();
    const rich = screen.getByRole('textbox', { name: 'Harness prompt' });
    expect(rich.querySelector('h1')).toHaveTextContent('Prompt prefix');
    expect(rich.querySelector('strong')).toHaveTextContent('careful');

    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByRole('textbox', {
      name: 'Harness prompt plain Markdown',
    }) as HTMLTextAreaElement;
    expect(plain).toHaveValue('# Prompt prefix\n\nUse **careful** guidance.');
    expect(screen.queryByRole('toolbar', { name: 'Markdown formatting' })).toBeNull();
    plain.focus();
    plain.setSelectionRange(2, 8);
    fireEvent.change(plain, {
      target: {
        value: '# Revised prefix\n\nUse **careful** guidance.',
        selectionStart: 9,
        selectionEnd: 9,
      },
    });
    expect(plain.selectionStart).toBe(9);

    fireEvent.click(screen.getByRole('button', { name: 'Markdown' }));
    expect(screen.getByRole('heading', { name: 'Revised prefix' })).toBeVisible();
    expect(screen.getByText('careful')).toHaveProperty('tagName', 'STRONG');
  });

  it('does not replace the rich surface or move its caret when a hash is typed', () => {
    render(<ControlledEditor />);
    const rich = screen.getByRole('textbox', { name: 'Harness prompt' });
    const heading = rich.querySelector('h1');
    expect(heading).not.toBeNull();
    if (!heading) return;
    const originalHeading = heading;
    heading.textContent = 'Prompt prefix#';
    const range = document.createRange();
    range.selectNodeContents(heading);
    range.collapse(false);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    rich.focus();

    fireEvent.input(rich, { inputType: 'insertText', data: '#' });

    expect(document.activeElement).toBe(rich);
    expect(rich.querySelector('h1')).toBe(originalHeading);
    expect(rich).toHaveTextContent('Prompt prefix#');
    expect(selection?.anchorNode && rich.contains(selection.anchorNode)).toBe(true);
  });

  it('renders a formatted read-only view when editing is unavailable', () => {
    render(<ControlledEditor editable={false} />);
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.queryByRole('button')).toBeNull();
  });
});
