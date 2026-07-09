import { LoaderCircle } from 'lucide-react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  busy?: boolean;
  leadingIcon?: ReactNode;
  trailingIcon?: ReactNode;
  variant?: ButtonVariant;
}

export function Button({
  busy = false,
  children,
  className,
  disabled,
  leadingIcon,
  trailingIcon,
  type = 'button',
  variant = 'secondary',
  ...props
}: ButtonProps) {
  const classes = ['ui-button', `ui-button--${variant}`, className].filter(Boolean).join(' ');
  const isDisabled = disabled || busy;

  return (
    <button
      {...props}
      aria-busy={busy || undefined}
      className={classes}
      disabled={isDisabled}
      type={type}
    >
      {busy ? <LoaderCircle aria-hidden="true" className="ui-spin" size={16} /> : leadingIcon}
      <span>{children}</span>
      {trailingIcon}
    </button>
  );
}
