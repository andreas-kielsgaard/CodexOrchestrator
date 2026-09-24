import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from 'react';
import type { CatalogOption } from '../CatalogSelect';
import './otpElementPicker.css';

export interface GroupedElementPickerNode {
  readonly id: string;
  readonly label: string;
  readonly items?: readonly CatalogOption[];
  readonly children?: readonly GroupedElementPickerNode[];
  readonly emptyLabel?: string;
}

function nodeItems(node: GroupedElementPickerNode): readonly CatalogOption[] {
  return [...(node.items ?? []), ...(node.children ?? []).flatMap(nodeItems)];
}

function expandableNodeIds(node: GroupedElementPickerNode): readonly string[] {
  return nodeItems(node).length
    ? [node.id, ...(node.children ?? []).flatMap(expandableNodeIds)]
    : [];
}

export function GroupedElementPicker({
  title,
  groups,
  selected,
  selectionMode,
  renderDetails,
  onConfirm,
  onClose,
}: {
  readonly title: string;
  readonly groups: readonly GroupedElementPickerNode[];
  readonly selected: readonly string[];
  readonly selectionMode: 'single' | 'multiple';
  renderDetails(id: string): ReactNode;
  onConfirm(ids: readonly string[]): void;
  onClose(): void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useId();
  const allItems = useMemo(() => groups.flatMap(nodeItems), [groups]);
  const [checked, setChecked] = useState(selected);
  const [preview, setPreview] = useState<string | null>(selected[0] ?? allItems[0]?.value ?? null);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(
    () => new Set(groups.flatMap(expandableNodeIds)),
  );
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const element = dialog.current;
    element?.showModal();
    return () => {
      element?.close();
      previous?.focus();
    };
  }, []);
  const missing = checked.filter((id) => !allItems.some((item) => item.value === id));
  const selectItems = (ids: readonly string[]) => {
    if (selectionMode === 'single') {
      const id = ids[0];
      if (id) setChecked([id]);
      return;
    }
    const enabled = ids.filter(
      (id) => !allItems.find((item) => item.value === id)?.disabled || checked.includes(id),
    );
    const allChecked = enabled.length > 0 && enabled.every((id) => checked.includes(id));
    setChecked((old) =>
      allChecked
        ? old.filter((id) => !enabled.includes(id))
        : [...old, ...enabled.filter((id) => !old.includes(id))],
    );
  };
  return (
    <dialog
      ref={dialog}
      className="otp-picker grouped-element-picker"
      aria-labelledby={heading}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onKeyDown={(event) => event.stopPropagation()}
    >
      <header>
        <h2 id={heading}>{title}</h2>
        <button type="button" aria-label="Close picker" onClick={onClose}>
          ×
        </button>
      </header>
      <div className="otp-picker__body">
        <nav aria-label={title}>
          {groups.map((group) => (
            <GroupedElementTreeNode
              key={group.id}
              node={group}
              checked={checked}
              expanded={expanded}
              preview={preview}
              selectionMode={selectionMode}
              onPreview={setPreview}
              onSelect={selectItems}
              onExpand={(id) =>
                setExpanded((old) => {
                  const next = new Set(old);
                  if (next.has(id)) next.delete(id);
                  else next.add(id);
                  return next;
                })
              }
            />
          ))}
          {!groups.length ? <p>No eligible elements are available.</p> : null}
          {missing.length ? (
            <div className="otp-picker__missing">
              <strong>Unavailable selections</strong>
              {missing.map((id) => (
                <div key={id}>
                  <span>{id}</span>
                  {selectionMode === 'multiple' ? (
                    <button
                      type="button"
                      onClick={() => setChecked((old) => old.filter((value) => value !== id))}
                    >
                      Remove
                    </button>
                  ) : null}
                </div>
              ))}
            </div>
          ) : null}
        </nav>
        <section className="otp-picker__details" aria-label="Element details">
          {preview && allItems.some((item) => item.value === preview) ? (
            renderDetails(preview)
          ) : (
            <p>Select an element to inspect its details.</p>
          )}
        </section>
      </div>
      <footer>
        <span>{selectionMode === 'multiple' ? `${checked.length} selected` : ''}</span>
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button
          type="button"
          disabled={selectionMode === 'single' && checked.length !== 1}
          onClick={() => onConfirm(checked)}
        >
          {selectionMode === 'single' ? title : 'Apply selection'}
        </button>
      </footer>
    </dialog>
  );
}

function GroupedElementTreeNode({
  node,
  checked,
  expanded,
  preview,
  selectionMode,
  onPreview,
  onSelect,
  onExpand,
}: {
  readonly node: GroupedElementPickerNode;
  readonly checked: readonly string[];
  readonly expanded: ReadonlySet<string>;
  readonly preview: string | null;
  readonly selectionMode: 'single' | 'multiple';
  onPreview(id: string): void;
  onSelect(ids: readonly string[]): void;
  onExpand(id: string): void;
}) {
  const items = nodeItems(node);
  const selectedCount = items.filter((item) => checked.includes(item.value)).length;
  const checkbox = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (checkbox.current)
      checkbox.current.indeterminate = selectedCount > 0 && selectedCount < items.length;
  }, [items.length, selectedCount]);
  if (!items.length) {
    return (
      <div className="grouped-element-picker__empty-group">
        <span>{node.label}</span>
        <small>{node.emptyLabel ?? 'No elements'}</small>
      </div>
    );
  }
  return (
    <div className="grouped-element-picker__node">
      <div className="grouped-element-picker__node-heading">
        {selectionMode === 'multiple' ? (
          <input
            ref={checkbox}
            type="checkbox"
            aria-label={`Toggle ${node.label}`}
            checked={selectedCount === items.length}
            onChange={() => onSelect(items.map((item) => item.value))}
          />
        ) : null}
        <button
          type="button"
          aria-expanded={expanded.has(node.id)}
          onClick={() => onExpand(node.id)}
        >
          <span aria-hidden="true">{expanded.has(node.id) ? '▾' : '▸'}</span> {node.label}
        </button>
      </div>
      {expanded.has(node.id) ? (
        <div className="grouped-element-picker__children">
          {node.children?.map((child) => (
            <GroupedElementTreeNode
              key={child.id}
              node={child}
              checked={checked}
              expanded={expanded}
              preview={preview}
              selectionMode={selectionMode}
              onPreview={onPreview}
              onSelect={onSelect}
              onExpand={onExpand}
            />
          ))}
          {node.items?.map((item) => (
            <div className="grouped-element-picker__item" key={item.value}>
              <input
                type={selectionMode === 'multiple' ? 'checkbox' : 'radio'}
                name={selectionMode === 'single' ? 'grouped-element-selection' : undefined}
                aria-label={`${selectionMode === 'multiple' ? 'Include' : 'Select'} ${item.label}`}
                checked={checked.includes(item.value)}
                disabled={item.disabled && !checked.includes(item.value)}
                onChange={() => onSelect([item.value])}
              />
              <button
                type="button"
                aria-pressed={preview === item.value}
                onClick={() => {
                  onPreview(item.value);
                  if (selectionMode === 'single') onSelect([item.value]);
                }}
              >
                {item.label}
                {item.disabled ? ' (unavailable)' : ''}
              </button>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
