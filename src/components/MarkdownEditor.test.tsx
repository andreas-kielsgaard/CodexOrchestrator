import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { MarkdownEditor } from './MarkdownEditor';

function ControlledEditor({ editable = true }: { readonly editable?: boolean }) {
  const [value, setValue] = useState('# Prompt prefix');
  return (
    <MarkdownEditor label="Harness prompt" value={value} editable={editable} onChange={setValue} />
  );
}

describe('MarkdownEditor', () => {
  it('switches explicitly between safe Markdown view and edit modes', () => {
    render(<ControlledEditor />);

    expect(screen.getByRole('heading', { name: 'Prompt prefix' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'View' })).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByLabelText('Harness prompt')).toHaveValue('# Prompt prefix');
    fireEvent.change(screen.getByLabelText('Harness prompt'), {
      target: { value: 'Updated text' },
    });
    const editor = screen.getByLabelText('Harness prompt') as HTMLTextAreaElement;
    editor.setSelectionRange(0, editor.value.length);
    fireEvent.click(screen.getByRole('button', { name: 'Bold' }));
    expect(screen.getByLabelText('Harness prompt')).toHaveValue('**Updated text**');
    fireEvent.click(screen.getByRole('button', { name: 'View' }));
    expect(screen.getByText('Updated text')).toBeVisible();
  });

  it('keeps edit mode unavailable when no product command boundary is present', () => {
    render(<ControlledEditor editable={false} />);
    expect(screen.getByRole('button', { name: 'Edit' })).toBeDisabled();
  });
});
