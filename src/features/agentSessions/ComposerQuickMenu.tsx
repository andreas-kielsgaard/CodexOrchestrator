import { useEffect, useRef } from 'react';
import { ArrowLeft, Check, ChevronRight } from 'lucide-react';
import type { useComposerQuickMenu } from './useComposerQuickMenu';
import './composerQuickMenu.css';

export function ComposerQuickMenu({
  menu,
  id,
}: {
  menu: ReturnType<typeof useComposerQuickMenu>;
  id: string;
}) {
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    list.current
      ?.querySelector('[data-highlighted="true"]')
      ?.scrollIntoView?.({ block: 'nearest' });
  }, [menu.selectedIndex]);
  if (!menu.open) return null;
  return (
    <div className="composer-quick-menu" onMouseDown={(event) => event.preventDefault()}>
      <div className="composer-quick-menu__heading">
        {menu.nested && (
          <button type="button" aria-label="Back to quick features" onClick={menu.back}>
            <ArrowLeft size={15} />
          </button>
        )}
        <strong>{menu.title}</strong>
        <button type="button" onClick={menu.dismiss} aria-label="Close quick features">
          Esc
        </button>
      </div>
      <div id={id} role="listbox" aria-label={menu.title} aria-busy={menu.loading} ref={list}>
        {menu.items.map((item, index) => (
          <div
            key={item.id}
            id={`${id}-${index}`}
            role="option"
            aria-selected={index === menu.selectedIndex}
            aria-disabled={Boolean(item.disabledReason || menu.unavailableReason)}
            data-highlighted={index === menu.selectedIndex}
            className="composer-quick-menu__option"
            onMouseMove={() => menu.setHighlight(index)}
            onClick={() => menu.choose(item)}
          >
            <span className="composer-quick-menu__check">
              {item.selected && <Check size={15} aria-label="Current choice" />}
            </span>
            <span>
              <strong>{item.label}</strong>
              <small>{item.disabledReason ?? item.description}</small>
            </span>
            {(item.children || item.loadChildren) && <ChevronRight size={15} aria-hidden="true" />}
          </div>
        ))}
      </div>
      {menu.loading && <p role="status">Loading available choices…</p>}
      {menu.error && (
        <div role="alert">
          <p>{menu.error}</p>
          <button type="button" onClick={menu.retry}>
            Retry
          </button>
        </div>
      )}
      {!menu.loading && !menu.error && !menu.items.length && (
        <p role="status">{menu.emptyMessage}</p>
      )}
      {menu.unavailableReason && <p role="status">{menu.unavailableReason}</p>}
      {menu.limitations.map((limitation) => (
        <p className="composer-quick-menu__limitation" key={limitation}>
          {limitation}
        </p>
      ))}
      <div className="composer-quick-menu__footer">
        ↑↓ navigate · Enter select · Esc back or close
      </div>
    </div>
  );
}
