import { useRef, type KeyboardEvent } from 'react';
import '../styles/sprintInformationSurfaces.css';

export type SprintWorkspaceTab = 'flow' | 'concerns' | 'documents';

const tabs: readonly { readonly id: SprintWorkspaceTab; readonly label: string }[] = [
  { id: 'flow', label: 'Flow' },
  { id: 'concerns', label: 'Concerns' },
  { id: 'documents', label: 'Documents' },
];

export function SprintWorkspaceTabs({
  selected,
  onSelect,
}: {
  readonly selected: SprintWorkspaceTab;
  readonly onSelect: (tab: SprintWorkspaceTab) => void;
}) {
  const refs = useRef(new Map<SprintWorkspaceTab, HTMLButtonElement>());

  const move = (event: KeyboardEvent<HTMLButtonElement>, nextIndex: number) => {
    event.preventDefault();
    const next = tabs[(nextIndex + tabs.length) % tabs.length];
    onSelect(next.id);
    refs.current.get(next.id)?.focus();
  };

  return (
    <div className="sprint-tabs" role="tablist" aria-label="Sprint information">
      {tabs.map((tab, index) => (
        <button
          key={tab.id}
          ref={(node) => {
            if (node) refs.current.set(tab.id, node);
            else refs.current.delete(tab.id);
          }}
          id={`sprint-${tab.id}-tab`}
          type="button"
          role="tab"
          aria-selected={selected === tab.id}
          aria-controls={`sprint-${tab.id}-panel`}
          tabIndex={selected === tab.id ? 0 : -1}
          onClick={() => onSelect(tab.id)}
          onKeyDown={(event) => {
            if (event.key === 'ArrowRight') move(event, index + 1);
            else if (event.key === 'ArrowLeft') move(event, index - 1);
            else if (event.key === 'Home') move(event, 0);
            else if (event.key === 'End') move(event, tabs.length - 1);
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
