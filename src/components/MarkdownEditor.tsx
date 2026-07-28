import { Bold, Code2, Heading2, Italic, Link, List, ListOrdered } from 'lucide-react';
import { useRef, useState } from 'react';
import { MarkdownContent } from './MarkdownContent';
import './markdownEditor.css';

export interface MarkdownEditorProps {
  readonly label: string;
  readonly value: string;
  readonly editable: boolean;
  onChange(value: string): void;
}

export function MarkdownEditor({ label, value, editable, onChange }: MarkdownEditorProps) {
  const [mode, setMode] = useState<'view' | 'edit'>('view');
  const textarea = useRef<HTMLTextAreaElement>(null);
  const apply = (prefix: string, suffix = prefix, fallback = 'text') => {
    const field = textarea.current;
    if (!field) return;
    const start = field.selectionStart;
    const end = field.selectionEnd;
    const selected = value.slice(start, end) || fallback;
    const next = `${value.slice(0, start)}${prefix}${selected}${suffix}${value.slice(end)}`;
    onChange(next);
    window.requestAnimationFrame(() => {
      field.focus();
      field.setSelectionRange(start + prefix.length, start + prefix.length + selected.length);
    });
  };

  return (
    <div className="markdown-editor">
      <div className="markdown-editor__mode" role="group" aria-label={`${label} mode`}>
        <button type="button" aria-pressed={mode === 'view'} onClick={() => setMode('view')}>
          View
        </button>
        <button
          type="button"
          aria-pressed={mode === 'edit'}
          disabled={!editable}
          onClick={() => setMode('edit')}
        >
          Edit
        </button>
      </div>
      {mode === 'edit' && editable ? (
        <div className="markdown-editor__editing">
          <div className="markdown-editor__toolbar" role="toolbar" aria-label="Markdown formatting">
            <FormatButton label="Heading" onClick={() => apply('## ', '', 'Heading')}>
              <Heading2 size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Bold" onClick={() => apply('**')}>
              <Bold size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Italic" onClick={() => apply('_')}>
              <Italic size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Bulleted list" onClick={() => apply('- ', '', 'List item')}>
              <List size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Numbered list" onClick={() => apply('1. ', '', 'List item')}>
              <ListOrdered size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Inline code" onClick={() => apply('`')}>
              <Code2 size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton label="Link" onClick={() => apply('[', '](https://)', 'link text')}>
              <Link size={16} aria-hidden="true" />
            </FormatButton>
          </div>
          <textarea
            ref={textarea}
            aria-label={label}
            rows={12}
            value={value}
            onChange={(event) => onChange(event.target.value)}
          />
        </div>
      ) : (
        <MarkdownContent className="markdown-editor__preview">{value}</MarkdownContent>
      )}
    </div>
  );
}

function FormatButton({
  label,
  onClick,
  children,
}: {
  readonly label: string;
  readonly onClick: () => void;
  readonly children: React.ReactNode;
}) {
  return (
    <button type="button" aria-label={label} title={label} onClick={onClick}>
      {children}
    </button>
  );
}
