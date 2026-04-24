import {
  type HTMLAttributes,
  forwardRef,
  useState,
  type ReactNode,
  Children,
  isValidElement,
  cloneElement,
} from 'react';

interface TabsProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  children: ReactNode;
  defaultTab?: string;
  onTabChange?: (tabId: string) => void;
}

interface TabListProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

interface TabProps extends HTMLAttributes<HTMLButtonElement> {
  id: string;
  active?: boolean;
  children: ReactNode;
}

interface TabPanelProps extends HTMLAttributes<HTMLDivElement> {
  id: string;
  active?: boolean;
  children: ReactNode;
}

function TabsComponent({
  children,
  defaultTab,
  onTabChange,
  className = '',
  ...props
}: TabsProps) {
  const [activeTab, setActiveTab] = useState(defaultTab ?? '');

  const handleTabChange = (tabId: string) => {
    setActiveTab(tabId);
    onTabChange?.(tabId);
  };

  return (
    <div className={['flex flex-col h-full', className].join(' ')} {...props}>
      {Children.map(children, (child) => {
        if (!isValidElement(child)) return child;
        if (child.type === TabList) {
          return cloneElement(child as React.ReactElement<TabListProps>, {
            children: Children.map((child.props as { children: ReactNode }).children, (tab) => {
              if (!isValidElement<TabProps>(tab) || tab.type !== Tab) return tab;
              return cloneElement(tab, {
                active: tab.props.id === activeTab,
                onClick: () => handleTabChange(tab.props.id),
              });
            }),
          });
        }
        if (child.type === TabPanel) {
          const panel = child as React.ReactElement<TabPanelProps>;
          return panel.props.id === activeTab ? panel : null;
        }
        return child;
      })}
    </div>
  );
}

const TabList = forwardRef<HTMLDivElement, TabListProps>(
  ({ children, className = '', ...props }, ref) => (
    <div
      ref={ref}
      role="tablist"
      className={[
        'flex gap-[var(--space-2xs)] p-[var(--space-2xs)]',
        'bg-[var(--color-track)] rounded-[var(--radius-md)]',
        'overflow-x-auto shrink-0',
        className,
      ].join(' ')}
      {...props}
    >
      {children}
    </div>
  ),
);

TabList.displayName = 'TabList';

const Tab = forwardRef<HTMLButtonElement, TabProps>(
  ({ active = false, children, className = '', ...props }, ref) => (
    <button
      ref={ref}
      role="tab"
      aria-selected={active}
      className={[
        'px-[var(--space-sm)] py-[var(--space-xs)]',
        'text-[var(--text-label)] font-[var(--weight-medium)]',
        'rounded-[var(--radius-sm)] select-none whitespace-nowrap',
        'transition-[background,color] duration-[var(--duration-fast)] ease-[var(--ease-out)]',
        active
          ? 'bg-[var(--color-bg-card)] text-[var(--color-text)] shadow-[0_1px_2px_rgba(0,0,0,0.1)]'
          : 'text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]',
        'focus-visible:outline-2 focus-visible:outline-[var(--color-border-focus)] focus-visible:outline-offset-1',
        className,
      ].join(' ')}
      {...props}
    >
      {children}
    </button>
  ),
);

Tab.displayName = 'Tab';

const TabPanel = forwardRef<HTMLDivElement, TabPanelProps>(
  ({ children, className = '', ...props }, ref) => (
    <div
      ref={ref}
      role="tabpanel"
      className={['flex-1 overflow-y-auto pt-[var(--space-lg)]', className].join(' ')}
      {...props}
    >
      {children}
    </div>
  ),
);

TabPanel.displayName = 'TabPanel';

export const Tabs = Object.assign(TabsComponent, { TabList, Tab, TabPanel });
