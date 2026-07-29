import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { MarkdownEditor } from './MarkdownEditor';

function ControlledEditor({
  editable = true,
  initial = '# Prompt prefix\n\nUse **careful** guidance.',
}: {
  readonly editable?: boolean;
  readonly initial?: string;
}) {
  const [value, setValue] = useState(initial);
  return (
    <MarkdownEditor label="Harness prompt" value={value} editable={editable} onChange={setValue} />
  );
}

async function selectBlockType(name: string) {
  const user = userEvent.setup();
  await user.click(screen.getByRole('combobox', { name: 'Block type' }));
  await user.click(await screen.findByRole('option', { name }));
}

describe('MarkdownEditor', () => {
  it('offers formatted Markdown and raw Plain modes over one working value', async () => {
    render(<ControlledEditor />);

    expect(screen.getByRole('button', { name: 'Markdown' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(
      await screen.findByRole('toolbar', { name: 'Markdown formatting' }, { timeout: 5_000 }),
    ).toBeVisible();
    const rich = await screen.findByRole('textbox', { name: 'Harness prompt' });
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
    expect(await screen.findByRole('heading', { name: 'Revised prefix' })).toBeVisible();
    expect(screen.getByText('careful')).toHaveProperty('tagName', 'STRONG');
  }, 10_000);

  it('applies and removes a heading on only the selected block', async () => {
    render(<ControlledEditor initial={'First block\n\nSecond # literal block'} />);
    const rich = await screen.findByRole('textbox', { name: 'Harness prompt' });
    const paragraphs = rich.querySelectorAll('p');
    expect(paragraphs).toHaveLength(2);

    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(paragraphs[0]);
    selection?.removeAllRanges();
    selection?.addRange(range);
    rich.focus();

    await selectBlockType('Heading 1');
    await waitFor(() => {
      expect(rich.querySelector('h1')).toHaveTextContent('First block');
      expect(rich.querySelectorAll('p')).toHaveLength(1);
    });
    expect(rich.querySelector('p')).toHaveTextContent('Second # literal block');

    const heading = rich.querySelector('h1');
    expect(heading).not.toBeNull();
    if (!heading) return;
    const headingRange = document.createRange();
    headingRange.selectNodeContents(heading);
    selection?.removeAllRanges();
    selection?.addRange(headingRange);
    rich.focus();
    await selectBlockType('Paragraph');
    await waitFor(() => {
      expect(rich.querySelector('h1')).toBeNull();
      expect(rich.querySelectorAll('p')).toHaveLength(2);
    });
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    expect(screen.getByLabelText('Harness prompt plain Markdown')).toHaveValue(
      'First block\n\nSecond # literal block',
    );
  });

  it('keeps the rich surface, caret, and literal hashes stable through mode round trips', async () => {
    render(<ControlledEditor initial="Literal # stays literal" />);
    const rich = await screen.findByRole('textbox', { name: 'Harness prompt' });
    const paragraph = rich.querySelector('p');
    expect(paragraph).not.toBeNull();
    if (!paragraph) return;
    const originalSurface = rich;
    const originalParagraph = paragraph;

    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByLabelText('Harness prompt plain Markdown') as HTMLTextAreaElement;
    plain.focus();
    plain.setSelectionRange(23, 23);
    fireEvent.change(plain, {
      target: {
        value: 'Literal # stays literal#',
        selectionStart: 24,
        selectionEnd: 24,
      },
    });

    expect(plain.selectionStart).toBe(24);
    fireEvent.click(screen.getByRole('button', { name: 'Markdown' }));
    await waitFor(() =>
      expect(rich.querySelector('p')).toHaveTextContent('Literal # stays literal#'),
    );
    expect(screen.getByRole('textbox', { name: 'Harness prompt' })).toBe(originalSurface);
    expect(rich.querySelector('p')).not.toBe(originalParagraph);

    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    expect(screen.getByLabelText('Harness prompt plain Markdown')).toHaveValue(
      'Literal # stays literal#',
    );
  });

  it('keeps the standard formatting controls accessible', async () => {
    render(<ControlledEditor />);
    const toolbar = await screen.findByRole('toolbar', { name: 'Markdown formatting' });
    expect(within(toolbar).getByRole('combobox', { name: 'Block type' })).toBeVisible();
    expect(within(toolbar).getByRole('radio', { name: 'Bold' })).toBeVisible();
    expect(within(toolbar).getByRole('radio', { name: 'Italic' })).toBeVisible();
  });

  it('renders a formatted read-only view when editing is unavailable', () => {
    render(<ControlledEditor editable={false} />);
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.queryByRole('button')).toBeNull();
  });
});
