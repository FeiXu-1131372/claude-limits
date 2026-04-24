import { type HTMLAttributes, forwardRef, type ReactNode } from 'react';

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  variant?: 'solid' | 'glass';
  hover?: boolean;
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ children, variant = 'solid', hover = false, className = '', ...props }, ref) => {
    const base =
      'rounded-[var(--radius-card)] border transition-[background,border-color] duration-[var(--duration-fast)] ease-[var(--ease-out)]';

    const variants = {
      solid: 'bg-[var(--color-bg-card)] border-[var(--color-border)]',
      glass: 'glass rounded-[var(--radius-card)]',
    };

    const hoverClass = hover
      ? 'hover:bg-[var(--color-bg-card-hover)] hover:border-[var(--color-border-hover)] cursor-pointer'
      : '';

    return (
      <div
        ref={ref}
        className={[base, variants[variant], hoverClass, className].join(' ')}
        {...props}
      >
        {children}
      </div>
    );
  },
);

Card.displayName = 'Card';

export const GlassCard = forwardRef<HTMLDivElement, CardProps>(
  (props, ref) => <Card ref={ref} variant="glass" {...props} />,
);

GlassCard.displayName = 'GlassCard';
