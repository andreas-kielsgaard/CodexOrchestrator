import {
  BlockTypeSelect,
  BoldItalicUnderlineToggles,
  CodeToggle,
  CreateLink,
  headingsPlugin,
  linkDialogPlugin,
  linkPlugin,
  listsPlugin,
  ListsToggle,
  markdownShortcutPlugin,
  MDXEditor,
  type MDXEditorMethods,
  quotePlugin,
  thematicBreakPlugin,
  toolbarPlugin,
  UndoRedo,
} from '@mdxeditor/editor';
import '@mdxeditor/editor/style.css';
import { useEffect, useLayoutEffect, useMemo, useRef } from 'react';

export function MdxMarkdownEditingSurface({
  label,
  value,
  active,
  onChange,
}: {
  readonly label: string;
  readonly value: string;
  readonly active: boolean;
  onChange(value: string): void;
}) {
  const editor = useRef<MDXEditorMethods>(null);
  const host = useRef<HTMLDivElement>(null);
  const lastRichValue = useRef(value);
  const plugins = useMemo(
    () => [
      headingsPlugin(),
      listsPlugin(),
      quotePlugin(),
      thematicBreakPlugin(),
      linkPlugin(),
      linkDialogPlugin(),
      markdownShortcutPlugin(),
      toolbarPlugin({
        toolbarClassName: 'markdown-editor__toolbar',
        toolbarContents: () => (
          <>
            <UndoRedo />
            <BlockTypeSelect />
            <BoldItalicUnderlineToggles options={['Bold', 'Italic']} />
            <ListsToggle options={['bullet', 'number']} />
            <CodeToggle />
            <CreateLink />
          </>
        ),
      }),
    ],
    [],
  );

  useEffect(() => {
    const root = host.current;
    if (!root) return;
    const applyAccessibleNames = () => {
      const contentEditable = root.querySelector<HTMLElement>('[contenteditable=true]');
      if (contentEditable?.getAttribute('aria-label') !== label)
        contentEditable?.setAttribute('aria-label', label);
      if (contentEditable?.getAttribute('aria-multiline') !== 'true')
        contentEditable?.setAttribute('aria-multiline', 'true');
      const toolbar = root.querySelector<HTMLElement>('[role=toolbar]');
      if (toolbar?.getAttribute('aria-label') !== 'Markdown formatting')
        toolbar?.setAttribute('aria-label', 'Markdown formatting');
    };
    applyAccessibleNames();
    const observer = new MutationObserver(applyAccessibleNames);
    observer.observe(root, { attributes: true, childList: true, subtree: true });
    return () => observer.disconnect();
  }, [label]);

  useLayoutEffect(() => {
    if (lastRichValue.current === value) return;
    lastRichValue.current = value;
    editor.current?.setMarkdown(value);
  }, [value]);

  useLayoutEffect(() => {
    if (!active) return;
    window.requestAnimationFrame(() => editor.current?.focus(undefined, { preventScroll: true }));
  }, [active]);

  return (
    <div ref={host}>
      <MDXEditor
        ref={editor}
        className="markdown-editor__mdx"
        contentEditableClassName="markdown-editor__rich"
        markdown={value}
        plugins={plugins}
        onChange={(next, initialMarkdownNormalize) => {
          if (initialMarkdownNormalize) return;
          lastRichValue.current = next;
          onChange(next);
        }}
      />
    </div>
  );
}
