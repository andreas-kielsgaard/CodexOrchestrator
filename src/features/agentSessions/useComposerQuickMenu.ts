import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import {
  filterQuickActions,
  sessionQuickActions,
  type ComposerQuickAction,
  type ComposerQuickFeatures,
} from './composerQuickActions';

export function useComposerQuickMenu(
  draft: string,
  onDraftChange: (value: string) => void,
  source: ComposerQuickFeatures | undefined,
  active: boolean,
  disabled: boolean,
) {
  const [path, setPath] = useState<string[]>([]);
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [catalog, setCatalog] = useState<{
    context: string;
    data: AgentSessionQuickFeatures;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const [notice, setNotice] = useState('');
  const [refresh, setRefresh] = useState(0);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const query = /^\/([^/\r\n]*)$/.exec(draft)?.[1];
  const open = Boolean(source && !disabled && query !== undefined && dismissed !== draft);
  const context = source?.contextKey;
  const load = source?.load;

  useEffect(() => {
    setPath([]);
    setDismissed(null);
    setNotice('');
    setCatalog(null);
  }, [context]);

  useEffect(() => {
    if (!open || !load || context === undefined) return;
    let current = true;
    setLoading(true);
    setError(null);
    setCatalog(null);
    void load()
      .then(
        (data) => {
          if (current) setCatalog({ context, data });
        },
        (cause) => {
          if (current) setError(cause instanceof Error ? cause.message : String(cause));
        },
      )
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [open, load, context, refresh]);

  useEffect(() => {
    if (query === undefined) {
      setPath([]);
      setDismissed(null);
    }
  }, [query]);
  useEffect(() => {
    setHighlight(0);
  }, [query, path, catalog]);
  useEffect(() => {
    if (!draft) setNotice('');
  }, [draft]);
  useEffect(() => {
    if (!open) return;
    const dismissOutside = (event: PointerEvent) => {
      if (
        event.target instanceof Node &&
        !textarea.current?.closest('form')?.contains(event.target)
      )
        setDismissed(draft);
    };
    document.addEventListener('pointerdown', dismissOutside);
    return () => document.removeEventListener('pointerdown', dismissOutside);
  }, [open, draft]);

  const data = catalog?.context === context ? catalog?.data : undefined;
  let actions = data && source ? sessionQuickActions(data, source, active) : [];
  let title = 'Quick features';
  let parent: ComposerQuickAction | undefined;
  for (const id of path) {
    parent = actions.find((item) => item.id === id);
    actions = parent?.children ?? [];
    title = parent?.label ?? title;
  }
  const items = filterQuickActions(actions, query ?? '');
  const selectedIndex = Math.min(highlight, Math.max(0, items.length - 1));
  const choose = (item: ComposerQuickAction | undefined) => {
    if (!item || loading || error || disabled || parent?.disabledReason || item.disabledReason)
      return;
    if (item.children) {
      setPath([...path, item.id]);
      onDraftChange('/');
    } else if (item.run) {
      const result = item.run();
      onDraftChange(result.replacement);
      setNotice(result.notice);
      setPath([]);
    }
    setHighlight(0);
    textarea.current?.focus();
  };
  const back = () => {
    const previous = path[path.length - 1];
    setPath(path.slice(0, -1));
    onDraftChange(`/${previous ?? ''}`);
    textarea.current?.focus();
  };
  const keyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!open || event.nativeEvent.isComposing || event.keyCode === 229) return false;
    if (event.key === 'Escape') {
      event.preventDefault();
      if (path.length) back();
      else setDismissed(draft);
      return true;
    }
    if (event.key === 'Backspace' && draft === '/' && path.length) {
      event.preventDefault();
      back();
      return true;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (items.length)
        setHighlight(
          (selectedIndex + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length,
        );
      return true;
    }
    if ((event.key === 'Enter' && !event.shiftKey) || event.key === 'Tab') {
      event.preventDefault();
      choose(items[selectedIndex]);
      return true;
    }
    return false;
  };
  return {
    open,
    title,
    items,
    selectedIndex,
    textarea,
    loading,
    error,
    notice,
    choose,
    back,
    keyDown,
    nested: path.length > 0,
    unavailableReason: parent?.disabledReason,
    limitations: data?.limitations ?? [],
    retry: () => setRefresh((value) => value + 1),
    dismiss: () => setDismissed(draft),
    accept: () => {
      textarea.current?.focus();
      choose(items[selectedIndex]);
    },
    setHighlight,
  };
}
