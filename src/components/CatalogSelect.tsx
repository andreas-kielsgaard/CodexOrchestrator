import type { ReactNode } from 'react';
import './catalogSelect.css';

export interface CatalogOption<T extends string = string> {
  readonly value: T;
  readonly label: string;
  readonly description?: string;
  readonly disabled?: boolean;
}

export interface CatalogState<T extends string = string> {
  readonly availability: 'available' | 'unavailable';
  readonly options: readonly CatalogOption<T>[];
  readonly sourceLabel?: string;
  readonly reason?: string;
}

interface CatalogFieldProps<T extends string> {
  readonly label: string;
  readonly catalog: CatalogState<T>;
  readonly disabled?: boolean;
  readonly hint?: ReactNode;
}

export interface CatalogSingleSelectProps<T extends string> extends CatalogFieldProps<T> {
  readonly value: T | null;
  readonly emptyLabel?: string;
  onChange(value: T | null): void;
}

/** Native single-value catalog control with explicit source and unavailable states. */
export function CatalogSingleSelect<T extends string>({
  label,
  catalog,
  value,
  disabled = false,
  emptyLabel = 'Not selected',
  hint,
  onChange,
}: CatalogSingleSelectProps<T>) {
  const unavailable = catalog.availability === 'unavailable';
  const missing = value !== null && !catalog.options.some((option) => option.value === value);

  return (
    <label className="catalog-field">
      <CatalogFieldHeading label={label} sourceLabel={catalog.sourceLabel} />
      <select
        aria-label={label}
        value={value ?? ''}
        disabled={disabled || unavailable}
        aria-invalid={missing || undefined}
        onChange={(event) => onChange((event.currentTarget.value || null) as T | null)}
      >
        <option value="">{emptyLabel}</option>
        {missing ? (
          <option value={value!} disabled>
            {value} (unavailable)
          </option>
        ) : null}
        {catalog.options.map((option) => (
          <option key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </option>
        ))}
      </select>
      <CatalogFieldSupport catalog={catalog} hint={hint} />
      {missing ? (
        <span className="catalog-field__support is-unavailable" role="alert">
          {label} is not available. Choose another value.
        </span>
      ) : null}
    </label>
  );
}

export interface CatalogMultiSelectProps<T extends string> extends CatalogFieldProps<T> {
  readonly values: readonly T[];
  onChange(values: readonly T[]): void;
}

/** Small checkbox-based multi-value catalog control suitable for bounded capability lists. */
export function CatalogMultiSelect<T extends string>({
  label,
  catalog,
  values,
  disabled = false,
  hint,
  onChange,
}: CatalogMultiSelectProps<T>) {
  const unavailable = catalog.availability === 'unavailable';

  return (
    <fieldset className="catalog-field" disabled={disabled || unavailable}>
      <legend>
        <CatalogFieldHeading label={label} sourceLabel={catalog.sourceLabel} />
      </legend>
      <div className="catalog-field__options">
        {catalog.options.map((option) => {
          const selected = values.includes(option.value);
          return (
            <label key={option.value} className="catalog-field__option">
              <input
                type="checkbox"
                value={option.value}
                checked={selected}
                disabled={option.disabled}
                onChange={() =>
                  onChange(
                    selected
                      ? values.filter((value) => value !== option.value)
                      : [...values, option.value],
                  )
                }
              />
              <span>
                <strong>{option.label}</strong>
                {option.description ? <small>{option.description}</small> : null}
              </span>
            </label>
          );
        })}
        {catalog.availability === 'available' && catalog.options.length === 0 ? (
          <p className="catalog-field__empty">No capabilities are exposed.</p>
        ) : null}
      </div>
      <CatalogFieldSupport catalog={catalog} hint={hint} />
    </fieldset>
  );
}

function CatalogFieldHeading({
  label,
  sourceLabel,
}: {
  readonly label: string;
  readonly sourceLabel?: string;
}) {
  return (
    <span className="catalog-field__heading">
      <span>{label}</span>
      {sourceLabel ? <small>{sourceLabel}</small> : null}
    </span>
  );
}

function CatalogFieldSupport<T extends string>({
  catalog,
  hint,
}: {
  readonly catalog: CatalogState<T>;
  readonly hint?: ReactNode;
}) {
  if (catalog.availability === 'unavailable')
    return (
      <span className="catalog-field__support is-unavailable" role="status">
        {catalog.reason ?? 'Catalog unavailable.'}
      </span>
    );
  return hint ? <span className="catalog-field__support">{hint}</span> : null;
}
