import type { HTMLAttributes, ReactNode } from 'react';

export interface PanelProps extends HTMLAttributes<HTMLElement> {
  actions?: ReactNode;
  children: ReactNode;
  eyebrow?: string;
  footer?: ReactNode;
  title?: string;
}

export function Panel({
  actions,
  children,
  className,
  eyebrow,
  footer,
  title,
  ...props
}: PanelProps) {
  const classes = ['ui-panel', className].filter(Boolean).join(' ');
  const hasHeader = title || eyebrow || actions;

  return (
    <section {...props} className={classes}>
      {hasHeader ? (
        <header className="ui-panel__header">
          <div>
            {eyebrow ? <p className="ui-panel__eyebrow">{eyebrow}</p> : null}
            {title ? <h2 className="ui-panel__title">{title}</h2> : null}
          </div>
          {actions ? <div className="ui-panel__actions">{actions}</div> : null}
        </header>
      ) : null}
      <div className="ui-panel__body">{children}</div>
      {footer ? <footer className="ui-panel__footer">{footer}</footer> : null}
    </section>
  );
}
