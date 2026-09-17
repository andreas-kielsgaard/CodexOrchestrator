import type { OtpConfigurationFieldDto } from '../../application/workflowAuthoring';

export function OtpConfigurationEditor({
  fields,
  value,
  onChange,
}: {
  readonly fields: readonly OtpConfigurationFieldDto[];
  readonly value: Readonly<Record<string, unknown>>;
  readonly onChange: (value: Readonly<Record<string, unknown>>) => void;
}) {
  const current = (key: string) =>
    String(value[key] ?? fields.find((field) => field.key === key)?.defaultValue ?? '');
  return (
    <div className="workflow-connection-editor__grid">
      {fields
        .filter((field) => !field.when || current(field.when[0]) === field.when[1])
        .map((field) => (
          <label key={field.key}>
            <span>{field.label}</span>
            {field.choices.length ? (
              <select
                value={current(field.key)}
                onChange={(event) => onChange({ ...value, [field.key]: event.currentTarget.value })}
              >
                {field.choices.map((choice) => (
                  <option key={choice} value={choice}>
                    {choice.replaceAll('_', ' ')}
                  </option>
                ))}
              </select>
            ) : (
              <input
                value={current(field.key)}
                onChange={(event) => onChange({ ...value, [field.key]: event.currentTarget.value })}
              />
            )}
          </label>
        ))}
    </div>
  );
}
