import { Bold, Code2, Heading2, Italic, Link, List, ListOrdered } from 'lucide-react';
import { marked } from 'marked';
import TurndownService from 'turndown';
import {
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
  type MouseEvent,
  type ReactNode,
} from 'react';
import { MarkdownContent } from './MarkdownContent';
import './markdownEditor.css';

export interface MarkdownEditorProps {
  readonly label: string;
  readonly value: string;
  readonly editable: boolean;
  onChange(value: string): void;
}

type EditorMode = 'markdown' | 'plain';

const turndown = new TurndownService({
  bulletListMarker: '-',
  codeBlockStyle: 'fenced',
  emDelimiter: '_',
  strongDelimiter: '**',
});

export function MarkdownEditor({ label, value, editable, onChange }: MarkdownEditorProps) {
  const [mode, setMode] = useState<EditorMode>('markdown');
  const richEditor = useRef<HTMLDivElement>(null);
  const plainEditor = useRef<HTMLTextAreaElement>(null);
  const lastEmittedValue = useRef<string | null>(null);
  const richCaretOffset = useRef(0);
  const plainVisited = useRef(false);
  const plainSelection = useRef({ start: 0, end: 0 });

  useLayoutEffect(() => {
    if (!editable || mode !== 'markdown' || !richEditor.current) return;
    const editor = richEditor.current;
    if (document.activeElement === editor && lastEmittedValue.current === value) return;
    const nextHtml = markdownToSafeHtml(value);
    if (editor.innerHTML !== nextHtml) editor.innerHTML = nextHtml;
  }, [editable, mode, value]);

  useLayoutEffect(() => {
    if (!editable || mode !== 'plain' || !plainEditor.current) return;
    const field = plainEditor.current;
    const { start, end } = plainSelection.current;
    if (document.activeElement === field)
      field.setSelectionRange(Math.min(start, value.length), Math.min(end, value.length));
  }, [editable, mode, value]);

  if (!editable)
    return <MarkdownContent className="markdown-editor__preview">{value}</MarkdownContent>;

  const emitRichValue = (event?: FormEvent<HTMLDivElement>) => {
    const editor = event?.currentTarget ?? richEditor.current;
    if (!editor) return;
    rememberRichSelection();
    const next = htmlToMarkdown(editor.innerHTML);
    lastEmittedValue.current = next;
    onChange(next);
  };

  const rememberRichSelection = () => {
    const selection = window.getSelection();
    if (!selection?.rangeCount || !richEditor.current) return;
    const range = selection.getRangeAt(0);
    if (!richEditor.current.contains(range.commonAncestorContainer)) return;
    const beforeCaret = range.cloneRange();
    beforeCaret.selectNodeContents(richEditor.current);
    beforeCaret.setEnd(range.endContainer, range.endOffset);
    richCaretOffset.current = beforeCaret.toString().length;
  };

  const restoreRichSelection = () => {
    const editor = richEditor.current;
    const selection = window.getSelection();
    if (!selection || !editor) return;
    const walker = document.createTreeWalker(editor, NodeFilter.SHOW_TEXT);
    let remaining = richCaretOffset.current;
    let node = walker.nextNode();
    while (node) {
      const length = node.textContent?.length ?? 0;
      if (remaining <= length) {
        const range = document.createRange();
        range.setStart(node, remaining);
        range.collapse(true);
        selection.removeAllRanges();
        selection.addRange(range);
        return;
      }
      remaining -= length;
      node = walker.nextNode();
    }
    const range = document.createRange();
    range.selectNodeContents(editor);
    range.collapse(false);
    selection.removeAllRanges();
    selection.addRange(range);
  };

  const switchMode = (next: EditorMode) => {
    if (next === mode) return;
    if (mode === 'markdown') {
      rememberRichSelection();
      if (!plainVisited.current) {
        const offset = Math.min(richCaretOffset.current, value.length);
        plainSelection.current = { start: offset, end: offset };
        plainVisited.current = true;
      }
    } else if (plainEditor.current) {
      plainSelection.current = {
        start: plainEditor.current.selectionStart,
        end: plainEditor.current.selectionEnd,
      };
    }
    setMode(next);
    window.requestAnimationFrame(() => {
      if (next === 'markdown') {
        richEditor.current?.focus();
        restoreRichSelection();
      } else {
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

  const runRichCommand = (command: string, argument?: string) => {
    richEditor.current?.focus();
    restoreRichSelection();
    document.execCommand?.(command, false, argument);
    rememberRichSelection();
    emitRichValue();
  };

  const wrapRichSelection = (tagName: 'code') => {
    richEditor.current?.focus();
    restoreRichSelection();
    const selection = window.getSelection();
    if (!selection?.rangeCount) return;
    const range = selection.getRangeAt(0);
    const wrapper = document.createElement(tagName);
    if (range.collapsed) {
      wrapper.textContent = 'code';
      range.insertNode(wrapper);
      range.selectNodeContents(wrapper);
    } else {
      wrapper.append(range.extractContents());
      range.insertNode(wrapper);
      range.selectNodeContents(wrapper);
    }
    selection.removeAllRanges();
    selection.addRange(range);
    rememberRichSelection();
    emitRichValue();
  };

  const keepSelection = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    rememberRichSelection();
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
      {mode === 'markdown' ? (
        <div className="markdown-editor__editing">
          <div className="markdown-editor__toolbar" role="toolbar" aria-label="Markdown formatting">
            <FormatButton
              label="Heading"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('formatBlock', 'h2')}
            >
              <Heading2 size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Bold"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('bold')}
            >
              <Bold size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Italic"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('italic')}
            >
              <Italic size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Bulleted list"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('insertUnorderedList')}
            >
              <List size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Numbered list"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('insertOrderedList')}
            >
              <ListOrdered size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Inline code"
              onMouseDown={keepSelection}
              onClick={() => wrapRichSelection('code')}
            >
              <Code2 size={16} aria-hidden="true" />
            </FormatButton>
            <FormatButton
              label="Link"
              onMouseDown={keepSelection}
              onClick={() => runRichCommand('createLink', 'https://')}
            >
              <Link size={16} aria-hidden="true" />
            </FormatButton>
          </div>
          <div
            ref={richEditor}
            className="markdown-editor__rich"
            contentEditable
            suppressContentEditableWarning
            role="textbox"
            aria-label={label}
            aria-multiline="true"
            onInput={emitRichValue}
            onKeyUp={rememberRichSelection}
            onMouseUp={rememberRichSelection}
            onFocus={restoreRichSelection}
            onBlur={rememberRichSelection}
          />
        </div>
      ) : (
        <textarea
          ref={plainEditor}
          className="markdown-editor__plain"
          aria-label={`${label} plain Markdown`}
          rows={12}
          value={value}
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
            lastEmittedValue.current = event.currentTarget.value;
            onChange(event.currentTarget.value);
          }}
        />
      )}
    </div>
  );
}

function FormatButton({
  label,
  onMouseDown,
  onClick,
  children,
}: {
  readonly label: string;
  onMouseDown(event: MouseEvent<HTMLButtonElement>): void;
  onClick(): void;
  readonly children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onMouseDown={onMouseDown}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function markdownToSafeHtml(value: string): string {
  const parsed = marked.parse(value, { async: false }) as string;
  const documentValue = new DOMParser().parseFromString(parsed, 'text/html');
  const allowedTags = new Set([
    'A',
    'BLOCKQUOTE',
    'BR',
    'CODE',
    'DEL',
    'EM',
    'H1',
    'H2',
    'H3',
    'H4',
    'H5',
    'H6',
    'HR',
    'LI',
    'OL',
    'P',
    'PRE',
    'STRONG',
    'UL',
  ]);
  for (const element of [...documentValue.body.querySelectorAll('*')]) {
    if (!allowedTags.has(element.tagName)) {
      element.replaceWith(...element.childNodes);
      continue;
    }
    for (const attribute of [...element.attributes]) {
      if (element.tagName !== 'A' || attribute.name !== 'href')
        element.removeAttribute(attribute.name);
    }
    if (
      element.tagName === 'A' &&
      /^(?:javascript|data):/i.test(element.getAttribute('href') ?? '')
    )
      element.removeAttribute('href');
  }
  return documentValue.body.innerHTML;
}

function htmlToMarkdown(value: string): string {
  return turndown.turndown(value).trimEnd();
}
