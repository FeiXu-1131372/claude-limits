import { type HTMLAttributes, forwardRef, type ReactNode } from 'react';

interface EmptyStateProps extends HTMLAttributes<HTMLDivElement> {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export const EmptyState = forwardRef<HTMLDivElement, EmptyStateProps>(
  ({ icon, title, description, action, className = '', ...props }, ref) => (
    <div
      ref={ref}
      className={[
        'flex flex-col items-center justify-center gap-[var(--space-sm)]',
        'py-[var(--space-4xl)] px-[var(--space-xl)]',
        'text-center',
        className,
      ].join(' ')}
      {...props}
    >
      {icon && (
        <div className="text-[color:var(--color-text-muted)] opacity-40 mb-[var(--space-xs)]">
          {icon}
        </div>
      )}
      <p className="text-[length:var(--text-body)] font-[var(--weight-medium)] text-[color:var(--color-text-secondary)]">
        {title}
      </p>
      {description && (
        <p className="text-[length:var(--text-micro)] text-[color:var(--color-text-muted)] max-w-[240px]">
          {description}
        </p>
      )}
      {action && <div className="mt-[var(--space-sm)]">{action}</div>}
    </div>
  ),
);

EmptyState.displayName = 'EmptyState';
