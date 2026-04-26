import { type HTMLAttributes, forwardRef, type ReactNode } from 'react';

type BadgeVariant = 'default' | 'accent' | 'safe' | 'warn' | 'danger' | 'live' | 'stale' | 'opus' | 'sonnet' | 'haiku';

interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: BadgeVariant;
  icon?: ReactNode;
  children: ReactNode;
}

const variantClasses: Record<BadgeVariant, string> = {
  default: 'bg-[var(--color-track)] text-[color:var(--color-text-secondary)]',
  accent: 'bg-[var(--color-accent-dim)] text-[color:var(--color-accent)]',
  safe: 'bg-[var(--color-safe-dim)] text-[color:var(--color-safe)]',
  warn: 'bg-[var(--color-warn-dim)] text-[color:var(--color-warn)]',
  danger: 'bg-[var(--color-danger-dim)] text-[color:var(--color-danger)]',
  live: 'bg-[var(--color-safe-dim)] text-[color:var(--color-safe)]',
  stale: 'bg-[var(--color-track)] text-[color:var(--color-text-muted)]',
  opus: 'bg-[var(--color-accent-dim)] text-[color:var(--color-accent)]',
  sonnet: 'bg-[var(--color-warn-dim)] text-[color:var(--color-warn)]',
  haiku: 'bg-[var(--color-safe-dim)] text-[color:var(--color-safe)]',
};

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ variant = 'default', icon, children, className = '', ...props }, ref) => (
    <span
      ref={ref}
      className={[
        'inline-flex items-center gap-[var(--space-2xs)]',
        'px-[7px] py-[2px]',
        'rounded-[var(--radius-pill)]',
        'text-[length:var(--text-micro)] font-[var(--weight-medium)]',
        'select-none',
        variantClasses[variant],
        className,
      ].join(' ')}
      {...props}
    >
      {variant === 'live' && (
        <span className="w-[5px] h-[5px] rounded-full bg-current animate-pulse" />
      )}
      {icon}
      {children}
    </span>
  ),
);

Badge.displayName = 'Badge';
