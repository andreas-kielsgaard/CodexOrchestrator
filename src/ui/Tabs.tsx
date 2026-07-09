import { useId, type ReactNode } from 'react';

export interface TabItem {
  disabled?: boolean;
  id: string;
  label: string;
  panel: ReactNode;
}

export interface TabsProps {
  ariaLabel: string;
  onChange: (tabId: string) => void;
  selectedTabId: string;
  tabs: TabItem[];
}

export function Tabs({ ariaLabel, onChange, selectedTabId, tabs }: TabsProps) {
  const baseId = useId();
  const selectedTab = tabs.find((tab) => tab.id === selectedTabId) ?? tabs[0];

  return (
    <div className="ui-tabs">
      <div aria-label={ariaLabel} className="ui-tabs__list" role="tablist">
        {tabs.map((tab) => {
          const isSelected = tab.id === selectedTab.id;
          const tabDomId = `${baseId}-${tab.id}-tab`;
          const panelDomId = `${baseId}-${tab.id}-panel`;

          return (
            <button
              aria-controls={panelDomId}
              aria-selected={isSelected}
              className="ui-tabs__tab"
              disabled={tab.disabled}
              id={tabDomId}
              key={tab.id}
              onClick={() => onChange(tab.id)}
              role="tab"
              tabIndex={isSelected ? 0 : -1}
              type="button"
            >
              {tab.label}
            </button>
          );
        })}
      </div>
      <div
        aria-labelledby={`${baseId}-${selectedTab.id}-tab`}
        className="ui-tabs__panel"
        id={`${baseId}-${selectedTab.id}-panel`}
        role="tabpanel"
      >
        {selectedTab.panel}
      </div>
    </div>
  );
}
