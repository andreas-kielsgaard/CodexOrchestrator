import { lazy, Suspense, useLayoutEffect, useRef, useState } from 'react';
import { MarkdownContent } from './MarkdownContent';
import './markdownEditor.css';

const MdxMarkdownEditingSurface = lazy(() =>
  import('./MdxMarkdownEditingSurface').then((module) => ({
    default: module.MdxMarkdownEditingSurface,
  })),
);

export interface MarkdownEditorProps {
  readonly label: string;
  readonly value: string;
  readonly editable: boolean;
  onChange(value: string): void;
}

type EditorMode = 'markdown' | 'plain';

export function MarkdownEditor({ label, value, editable, onChange }: MarkdownEditorProps) {
  const [mode, setMode] = useState<EditorMode>('markdown');
  const plainEditor = useRef<HTMLTextAreaElement>(null);
  const plainSelection = useRef({ start: 0, end: 0 });

  useLayoutEffect(() => {
    if (!editable || mode !== 'plain' || !plainEditor.current) return;
    const field = plainEditor.current;
    const { start, end } = plainSelection.current;
    field.setSelectionRange(Math.min(start, value.length), Math.min(end, value.length));
  }, [editable, mode, value]);

  if (!editable)
    return <MarkdownContent className="markdown-editor__preview">{value}</MarkdownContent>;

  const switchMode = (next: EditorMode) => {
    if (next === mode) return;
    if (mode === 'plain' && plainEditor.current)
      plainSelection.current = {
        start: plainEditor.current.selectionStart,
        end: plainEditor.current.selectionEnd,
      };
    setMode(next);
    window.requestAnimationFrame(() => {
      if (next === 'plain') {
        const field = plainEditor.current;
        if (!field) return;
        field.focus();
        field.setSelectionRange(
          Math.min(plainSelection.current.start, field.value.length),
          Math.min(plainSelection.current.end, field.value.length),
        );
      }
    });
  };

  return (
    <div className="markdown-editor">
      <div className="markdown-editor__mode" role="group" aria-label={`${label} mode`}>
        <button
          type="button"
          aria-pressed={mode === 'markdown'}
          onClick={() => switchMode('markdown')}
        >
          Markdown
        </button>
        <button type="button" aria-pressed={mode === 'plain'} onClick={() => switchMode('plain')}>
          Plain
        </button>
      </div>
      <div className="markdown-editor__rich-host" hidden={mode !== 'markdown'}>
        <Suspense fallback={<p className="markdown-editor__loading">Loading editor…</p>}>
          <MdxMarkdownEditingSurface
            label={label}
            value={value}
            active={mode === 'markdown'}
            onChange={onChange}
          />
        </Suspense>
      </div>
      <textarea
        ref={plainEditor}
        className="markdown-editor__plain"
        aria-label={`${label} plain Markdown`}
        rows={12}
        value={value}
        hidden={mode !== 'plain'}
        onSelect={(event) => {
          plainSelection.current = {
            start: event.currentTarget.selectionStart,
            end: event.currentTarget.selectionEnd,
          };
        }}
        onChange={(event) => {
          plainSelection.current = {
            start: event.currentTarget.selectionStart,
            end: event.currentTarget.selectionEnd,
          };
          onChange(event.currentTarget.value);
        }}
      />
    </div>
  );
}
