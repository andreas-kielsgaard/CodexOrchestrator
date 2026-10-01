import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import {
  filterQuickActions,
  sessionQuickActions,
  type ComposerQuickFeatures,
} from './composerQuickActions';
import type { ComposerQuickAction, ComposerQuickPage } from './composerQuickMenuTypes';
import { composerTargetActions, type ComposerTargetSource } from './composerTargetActions';
import { commandToken, preferredCommandIndex } from './composerCommandToken';

interface NavigationFrame {
  readonly id: string;
  readonly query: string;
  readonly start: number;
  readonly caret: number;
  readonly token: number;
  readonly acceptsSlash?: boolean;
  readonly load?: () => Promise<ComposerQuickPage>;
  readonly page?: ComposerQuickPage;
  readonly loading?: boolean;
  readonly error?: string;
}

export function useComposerQuickMenu(
  draft: string,
  onDraftChange: (value: string) => void,
  source: ComposerQuickFeatures | undefined,
  disabled: boolean,
  targets?: ComposerTargetSource,
) {
  const [frames, setFrames] = useState<readonly NavigationFrame[]>([]);
  const sequence = useRef(0);
  const [dismissed, setDismissed] = useState<number | null>(null);
  const [caret, setCaret] = useState(draft.length);
  const [catalog, setCatalog] = useState<{
    context: string;
    data: AgentSessionQuickFeatures;
  } | null>(null);
  const [nativeError, setNativeError] = useState<string | null>(null);
  const [nativeLoading, setNativeLoading] = useState(false);
  const [highlight, setHighlight] = useState<number | null>(null);
  const [notice, setNotice] = useState('');
  const [refresh, setRefresh] = useState(0);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const context = JSON.stringify([source?.contextKey, targets?.contextKey]);
  const [navigationContext, setNavigationContext] = useState(context);
  const currentFrames = navigationContext === context ? frames : [];
  const frame = currentFrames.at(-1);
  const token = commandToken(
    draft,
    caret,
    frame?.acceptsSlash ? frame.start : undefined,
    Boolean(frame),
  );
  const query = token?.query;
  const open = Boolean(
    (source || targets) && !disabled && token !== null && dismissed !== token.start,
  );
  const load = source?.load;
  const nativeContext = source?.contextKey;
  const noticeContext = targets?.contextKey ?? nativeContext;

  useEffect(() => {
    setFrames([]);
    setNavigationContext(context);
    setDismissed(null);
    setCatalog(null);
    setNativeError(null);
  }, [context]);
  useEffect(() => {
    setNotice('');
  }, [noticeContext]);

  useEffect(() => {
    if (!open || !load || nativeContext === undefined || source?.catalogue) return;
    let current = true;
    setNativeLoading(true);
    setNativeError(null);
    void load()
      .then(
        (data) => {
          if (current) setCatalog({ context: nativeContext, data });
        },
        (cause) => {
          if (current) setNativeError(cause instanceof Error ? cause.message : String(cause));
        },
      )
      .finally(() => {
        if (current) setNativeLoading(false);
      });
    return () => {
      current = false;
    };
  }, [open, load, nativeContext, refresh, source?.catalogue]);

  useEffect(() => {
    if (!source?.catalogue || nativeContext === undefined) return;
    setCatalog({ context: nativeContext, data: source.catalogue });
  }, [source?.catalogue, nativeContext]);

  const frameToken = frame?.token;
  const pageLoader = frame?.load;
  const pageReady = Boolean(frame?.page);
  useEffect(() => {
    if (!open || !pageLoader || pageReady || frameToken === undefined) return;
    let current = true;
    const update = (change: Partial<NavigationFrame>) => {
      if (!current) return;
      setFrames((previous) =>
        previous.at(-1)?.token === frameToken
          ? [...previous.slice(0, -1), { ...previous[previous.length - 1], ...change }]
          : previous,
      );
    };
    update({ loading: true, error: undefined });
    void pageLoader().then(
      (page) => update({ page, loading: false }),
      (cause) =>
        update({ error: cause instanceof Error ? cause.message : String(cause), loading: false }),
    );
    return () => {
      current = false;
    };
  }, [open, pageLoader, pageReady, frameToken, context]);

  useEffect(() => {
    if (query === undefined) {
      setFrames([]);
      setDismissed(null);
    }
  }, [query]);
  useEffect(() => {
    setHighlight(null);
  }, [query, frameToken, frame?.page, catalog]);
  useEffect(() => {
    if (!open) return;
    const dismissOutside = (event: PointerEvent) => {
      if (
        event.target instanceof Node &&
        !textarea.current?.closest('form')?.contains(event.target)
      ) {
        setDismissed(draft.lastIndexOf('/', caret - 1));
        setFrames([]);
      }
    };
    document.addEventListener('pointerdown', dismissOutside);
    return () => document.removeEventListener('pointerdown', dismissOutside);
  }, [open, draft, caret]);

  const data = catalog && catalog.context === nativeContext ? catalog.data : undefined;
  let actions: readonly ComposerQuickAction[] = [
    ...(targets ? composerTargetActions(targets) : []),
    ...(data && source ? sessionQuickActions(data, source) : []),
  ];
  let title = 'Quick features';
  let unavailableReason: string | undefined;
  for (const step of currentFrames) {
    const parent = actions.find((item) => item.id === step.id);
    unavailableReason = unavailableReason ?? parent?.disabledReason;
    title = step.page?.title ?? parent?.label ?? title;
    actions = step.load ? (step.page?.items ?? []) : (parent?.children ?? []);
  }
  const items = filterQuickActions(actions, query ?? '');
  const selectedIndex = Math.min(
    highlight ?? preferredCommandIndex(items, query ?? ''),
    Math.max(0, items.length - 1),
  );
  const replaceToken = (replacement: string) => {
    if (!token) return;
    onDraftChange(draft.slice(0, token.start) + replacement + draft.slice(token.end));
    const nextCaret = token.start + replacement.length;
    setCaret(nextCaret);
    requestAnimationFrame(() => textarea.current?.setSelectionRange(nextCaret, nextCaret));
  };
  const loading = frame ? Boolean(frame.loading) : Boolean(source && nativeLoading);
  const error = frame ? frame.error : nativeError;
  const choose = (item: ComposerQuickAction | undefined) => {
    if (
      !item ||
      disabled ||
      unavailableReason ||
      item.disabledReason ||
      (frame && (loading || error))
    )
      return;
    if (item.children || item.loadChildren) {
      setFrames([
        ...currentFrames,
        {
          id: item.id,
          query: draft,
          start: token?.start ?? 0,
          caret,
          token: ++sequence.current,
          acceptsSlash: item.acceptsSlash,
          load: item.loadChildren,
          loading: Boolean(item.loadChildren),
        },
      ]);
      replaceToken('/');
    } else if (item.run) {
      const result = item.run();
      replaceToken(result.replacement);
      setNotice(result.notice);
      setFrames([]);
      setDismissed(token?.start ?? null);
    }
    setHighlight(null);
    textarea.current?.focus();
  };
  const back = () => {
    setFrames(currentFrames.slice(0, -1));
    onDraftChange(frame?.query ?? '/');
    setCaret(frame?.caret ?? 1);
    textarea.current?.focus();
  };
  const dismiss = () => {
    setDismissed(draft.lastIndexOf('/', caret - 1));
    setFrames([]);
    textarea.current?.focus();
  };
  const keyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!open || event.nativeEvent.isComposing || event.keyCode === 229) return false;
    if (event.key === 'Escape') {
      event.preventDefault();
      dismiss();
      return true;
    }
    if (event.key === 'Backspace' && query === '' && currentFrames.length) {
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
    nested: currentFrames.length > 0,
    unavailableReason,
    emptyMessage: query?.trim()
      ? 'No matching choices'
      : (frame?.page?.emptyMessage ?? 'No matching choices'),
    limitations: frame ? (frame.page?.limitations ?? []) : (data?.limitations ?? []),
    retry: () => {
      if (frame?.load)
        setFrames([
          ...currentFrames.slice(0, -1),
          { ...frame, token: ++sequence.current, page: undefined, error: undefined, loading: true },
        ]);
      else if (source?.refresh) {
        setNativeLoading(true);
        setNativeError(null);
        void source
          .refresh()
          .then(
            (data) => setCatalog({ context: source.contextKey, data }),
            (cause) => setNativeError(cause instanceof Error ? cause.message : String(cause)),
          )
          .finally(() => setNativeLoading(false));
      } else setRefresh((value) => value + 1);
    },
    dismiss,
    accept: () => {
      textarea.current?.focus();
      choose(items[selectedIndex]);
    },
    setHighlight,
    setCaret,
  };
}
