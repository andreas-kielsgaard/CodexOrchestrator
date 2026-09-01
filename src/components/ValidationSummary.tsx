import './validationSummary.css';

export interface ValidationSummaryProps {
  readonly errors: readonly string[];
  readonly title?: string;
}

/** Compact error summary for controlled editors. Validation remains owned by the caller. */
export function ValidationSummary({
  errors,
  title = 'Resolve these issues before saving',
}: ValidationSummaryProps) {
  if (!errors.length) return null;
  return (
    <section className="validation-summary" role="alert" aria-label="Validation errors">
      <strong>{title}</strong>
      <ul>
        {errors.map((error, index) => (
          <li key={`${index}-${error}`}>{error}</li>
        ))}
      </ul>
    </section>
  );
}
