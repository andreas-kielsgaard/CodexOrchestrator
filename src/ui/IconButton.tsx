import { LoaderCircle, type LucideIcon } from 'lucide-react';
import type { ButtonHTMLAttributes } from 'react';

export type IconButtonVariant = 'default' | 'quiet' | 'danger';

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  busy?: boolean;
  icon: LucideIcon;
  label: string;
  variant?: IconButtonVariant;
}

export function IconButton({
  busy = false,
  className,
  disabled,
  icon: Icon,
  label,
  title = label,
  type = 'button',
  variant = 'default',
  ...props
}: IconButtonProps) {
  const classes = ['ui-icon-button', `ui-icon-button--${variant}`, className]
    .filter(Boolean)
    .join(' ');
  const isDisabled = disabled || busy;

  return (
    <button
      {...props}
      aria-busy={busy || undefined}
      aria-label={label}
      className={classes}
      disabled={isDisabled}
      title={title}
      type={type}
    >
      {busy ? (
        <LoaderCircle aria-hidden="true" className="ui-spin" size={16} />
      ) : (
        <Icon aria-hidden="true" size={16} />
      )}
    </button>
  );
}
