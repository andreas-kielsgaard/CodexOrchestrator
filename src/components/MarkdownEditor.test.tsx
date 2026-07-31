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

describe('MarkdownEditor', () => {
  it('reuses the File Review AgentMarkdown renderer for GFM and safe HTML boundaries', () => {
    const { container } = render(
      <ControlledEditor
        initial={`# Prompt prefix

- [x] GFM task

| Name | State |
| --- | --- |
| Session | ready |

[Source](https://example.com)

<script>alert("unsafe")</script><strong>raw</strong>`}
      />,
    );

    expect(screen.getByRole('button', { name: 'Rendered' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    const rendered = screen.getByRole('region', {
      name: 'Harness prompt rendered Markdown',
    });
    expect(rendered.querySelector('.agent-markdown.markdown-editor__preview')).not.toBeNull();
    expect(within(rendered).getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(within(rendered).getByRole('checkbox')).toBeChecked();
    expect(within(rendered).getByRole('table')).toBeVisible();
    expect(within(rendered).getByRole('link', { name: 'Source' })).toHaveAttribute(
      'target',
      '_blank',
    );
    expect(within(rendered).getByRole('link', { name: 'Source' })).toHaveAttribute(
      'rel',
      'noreferrer',
    );
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('strong')).toBeNull();
    expect(container.querySelector('[contenteditable=true]')).toBeNull();
    expect(screen.queryByRole('toolbar')).toBeNull();
  });

  it('round-trips exact source and preserves its selection across rendered mode', async () => {
    const initial = '## Exact source\n\nLiteral # hash  \n\n- [x] keep\n';
    render(<ControlledEditor initial={initial} />);

    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByLabelText('Harness prompt plain Markdown') as HTMLTextAreaElement;
    expect(plain).toHaveValue(initial);
    plain.focus();
    plain.setSelectionRange(3, 15);
    fireEvent.select(plain);

    fireEvent.click(screen.getByRole('button', { name: 'Rendered' }));
    expect(screen.getByRole('heading', { name: 'Exact source' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));

    await waitFor(() => {
      expect(plain).toHaveValue(initial);
      expect(plain.selectionStart).toBe(3);
      expect(plain.selectionEnd).toBe(15);
    });
  });

  it('updates only from Plain source and keeps text and caret stable after changes', async () => {
    render(<ControlledEditor initial="Literal # stays literal" />);
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));
    const plain = screen.getByLabelText('Harness prompt plain Markdown') as HTMLTextAreaElement;
    const revised = '# Revised prefix\n\nLiteral # stays literal#\n';

    fireEvent.change(plain, {
      target: {
        value: revised,
        selectionStart: 18,
        selectionEnd: 18,
      },
    });
    expect(plain).toHaveValue(revised);
    expect(plain.selectionStart).toBe(18);

    fireEvent.click(screen.getByRole('button', { name: 'Rendered' }));
    expect(screen.getByRole('heading', { name: 'Revised prefix' })).toBeVisible();
    expect(screen.getByText('Literal # stays literal#')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Plain' }));

    await waitFor(() => {
      expect(plain).toHaveValue(revised);
      expect(plain.selectionStart).toBe(18);
      expect(plain.selectionEnd).toBe(18);
    });
  });

  it('keeps the two standard mode controls and Plain source keyboard accessible', async () => {
    const user = userEvent.setup();
    render(<ControlledEditor />);
    const modes = screen.getByRole('group', { name: 'Harness prompt mode' });
    const rendered = within(modes).getByRole('button', { name: 'Rendered' });
    const plainMode = within(modes).getByRole('button', { name: 'Plain' });

    expect(rendered).toHaveAttribute('type', 'button');
    expect(plainMode).toHaveAttribute('type', 'button');
    await user.click(plainMode);
    const plain = screen.getByRole('textbox', { name: 'Harness prompt plain Markdown' });
    await waitFor(() => expect(plain).toHaveFocus());
    expect(screen.queryByRole('toolbar')).toBeNull();
  });

  it('renders the canonical formatted view when editing is unavailable', () => {
    const { container } = render(<ControlledEditor editable={false} />);
    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(container.querySelector('.agent-markdown.markdown-editor__preview')).not.toBeNull();
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});
