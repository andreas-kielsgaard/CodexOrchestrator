import type { ReactNode } from 'react';
import './productViewHeader.css';

export interface ProductViewHeaderProps {
  readonly context: string;
  readonly title: string;
  readonly actions?: ReactNode;
  readonly actionsLabel?: string;
}

/** Shared product-level identity with view actions kept outside working panels. */
export function ProductViewHeader({
  context,
  title,
  actions,
  actionsLabel = 'View actions',
}: ProductViewHeaderProps) {
  return (
    <header className="product-view-header">
      <div className="product-view-header__title">
        <span>{context}</span>
        <h1 title={title}>{title}</h1>
      </div>
      {actions && (
        <div className="product-view-header__actions" role="group" aria-label={actionsLabel}>
          {actions}
        </div>
      )}
    </header>
  );
}
