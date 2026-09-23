import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from 'react';
import type { CatalogOption } from '../../components/CatalogSelect';
import '../../components/otp/otpElementPicker.css';

export interface CapabilityPickerNode {
  readonly id: string;
  readonly label: string;
  readonly items?: readonly CatalogOption[];
  readonly children?: readonly CapabilityPickerNode[];
}

function nodeItems(node: CapabilityPickerNode): readonly CatalogOption[] {
  return [...(node.items ?? []), ...(node.children ?? []).flatMap(nodeItems)];
}

function nodeIds(node: CapabilityPickerNode): readonly string[] {
  return [node.id, ...(node.children ?? []).flatMap(nodeIds)];
}

export function GroupedCapabilityPicker({
  title,
  groups,
  selected,
  renderDetails,
  onConfirm,
  onClose,
}: {
  readonly title: string;
  readonly groups: readonly CapabilityPickerNode[];
  readonly selected: readonly string[];
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
    () => new Set(groups.flatMap(nodeIds)),
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
  const toggleMany = (ids: readonly string[]) => {
    const enabled = ids.filter(
      (id) => !allItems.find((item) => item.value === id)?.disabled || checked.includes(id),
    );
    const allChecked = enabled.every((id) => checked.includes(id));
    setChecked((old) =>
      allChecked
        ? old.filter((id) => !enabled.includes(id))
        : [...old, ...enabled.filter((id) => !old.includes(id))],
    );
  };
  return (
    <dialog
      ref={dialog}
      className="otp-picker capability-picker"
      aria-labelledby={heading}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
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
            <CapabilityTreeNode
              key={group.id}
              node={group}
              checked={checked}
              expanded={expanded}
              preview={preview}
              onPreview={setPreview}
              onToggle={toggleMany}
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
          {!allItems.length ? <p>No eligible elements are available.</p> : null}
          {missing.length ? (
            <div className="otp-picker__missing">
              <strong>Unavailable selections</strong>
              {missing.map((id) => (
                <div key={id}>
                  <span>{id}</span>
                  <button type="button" onClick={() => setChecked((old) => old.filter((x) => x !== id))}>
                    Remove
                  </button>
                </div>
              ))}
            </div>
          ) : null}
        </nav>
        <section className="otp-picker__details" aria-label="Element details">
          {preview ? renderDetails(preview) : <p>Select an element to inspect its details.</p>}
        </section>
      </div>
      <footer>
        <span>{checked.length} selected</span>
        <button type="button" onClick={onClose}>Cancel</button>
        <button type="button" onClick={() => onConfirm(checked)}>Apply selection</button>
      </footer>
    </dialog>
  );
}

function CapabilityTreeNode({
  node,
  checked,
  expanded,
  preview,
  onPreview,
  onToggle,
  onExpand,
}: {
  readonly node: CapabilityPickerNode;
  readonly checked: readonly string[];
  readonly expanded: ReadonlySet<string>;
  readonly preview: string | null;
  onPreview(id: string): void;
  onToggle(ids: readonly string[]): void;
  onExpand(id: string): void;
}) {
  const items = nodeItems(node);
  const selectedCount = items.filter((item) => checked.includes(item.value)).length;
  const checkbox = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (checkbox.current) checkbox.current.indeterminate = selectedCount > 0 && selectedCount < items.length;
  }, [items.length, selectedCount]);
  const hasChildren = Boolean(node.children?.length || node.items?.length);
  return (
    <div className="capability-picker__node">
      <div className="capability-picker__node-heading">
        <input
          ref={checkbox}
          type="checkbox"
          aria-label={`Toggle ${node.label}`}
          checked={items.length > 0 && selectedCount === items.length}
          onChange={() => onToggle(items.map((item) => item.value))}
        />
        <button type="button" aria-expanded={expanded.has(node.id)} onClick={() => onExpand(node.id)}>
          <span aria-hidden="true">{expanded.has(node.id) ? '▾' : '▸'}</span> {node.label}
        </button>
      </div>
      {hasChildren && expanded.has(node.id) ? (
        <div className="capability-picker__children">
          {node.children?.map((child) => (
            <CapabilityTreeNode
              key={child.id}
              node={child}
              checked={checked}
              expanded={expanded}
              preview={preview}
              onPreview={onPreview}
              onToggle={onToggle}
              onExpand={onExpand}
            />
          ))}
          {node.items?.map((item) => (
            <div className="capability-picker__item" key={item.value}>
              <input
                type="checkbox"
                aria-label={`Include ${item.label}`}
                checked={checked.includes(item.value)}
                disabled={item.disabled && !checked.includes(item.value)}
                onChange={() => onToggle([item.value])}
              />
              <button
                type="button"
                aria-pressed={preview === item.value}
                onClick={() => onPreview(item.value)}
              >
                {item.label}{item.disabled ? ' (unavailable)' : ''}
              </button>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
