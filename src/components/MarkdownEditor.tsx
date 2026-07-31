import { useLayoutEffect, useRef, useState } from 'react';
import { AgentMarkdown } from '../features/agentSessions/AgentMarkdown';
import './markdownEditor.css';

export interface MarkdownEditorProps {
  readonly label: string;
  readonly value: string;
  readonly editable: boolean;
  onChange(value: string): void;
}

type EditorMode = 'rendered' | 'plain';

export function MarkdownEditor({ label, value, editable, onChange }: MarkdownEditorProps) {
  const [mode, setMode] = useState<EditorMode>('rendered');
  const plainEditor = useRef<HTMLTextAreaElement>(null);
  const plainSelection = useRef({ start: 0, end: 0 });

  useLayoutEffect(() => {
    if (!editable || mode !== 'plain' || !plainEditor.current) return;
    const field = plainEditor.current;
    const { start, end } = plainSelection.current;
    field.setSelectionRange(Math.min(start, value.length), Math.min(end, value.length));
  }, [editable, mode, value]);

  if (!editable) return <AgentMarkdown className="markdown-editor__preview">{value}</AgentMarkdown>;

  const switchMode = (next: EditorMode) => {
    if (next === mode) return;
    if (mode === 'plain' && plainEditor.current)
      plainSelection.current = {
        start: plainEditor.current.selectionStart,
        end: plainEditor.current.selectionEnd,
      };
    setMode(next);
    window.requestAnimationFrame(() => {
      if (next !== 'plain') return;
      const field = plainEditor.current;
      if (!field) return;
      field.focus();
      field.setSelectionRange(
        Math.min(plainSelection.current.start, field.value.length),
        Math.min(plainSelection.current.end, field.value.length),
      );
    });
  };

  return (
    <div className="markdown-editor">
      <div className="markdown-editor__mode" role="group" aria-label={`${label} mode`}>
        <button
          type="button"
          aria-pressed={mode === 'rendered'}
          onClick={() => switchMode('rendered')}
        >
          Rendered
        </button>
        <button type="button" aria-pressed={mode === 'plain'} onClick={() => switchMode('plain')}>
          Plain
        </button>
      </div>
      <div
        className="markdown-editor__rendered"
        role="region"
        aria-label={`${label} rendered Markdown`}
        hidden={mode !== 'rendered'}
      >
        <AgentMarkdown className="markdown-editor__preview">{value}</AgentMarkdown>
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
