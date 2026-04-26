import { type ButtonHTMLAttributes, forwardRef, type ReactNode } from 'react';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  label: string;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ children, label, className = '', ...props }, ref) => (
    <button
      ref={ref}
      aria-label={label}
      className={[
        'w-[30px] h-[30px] inline-flex items-center justify-center',
        'rounded-[var(--radius-sm)]',
        'text-[color:var(--color-text-muted)]',
        'transition-[background,color] duration-[var(--duration-fast)] ease-[var(--ease-out)]',
        'hover:bg-[var(--color-bg-card)] hover:text-[color:var(--color-text-secondary)]',
        'active:opacity-70',
        'focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-1',
        'disabled:opacity-30 disabled:pointer-events-none',
        className,
      ].join(' ')}
      {...props}
    >
      {children}
    </button>
  ),
);

IconButton.displayName = 'IconButton';
