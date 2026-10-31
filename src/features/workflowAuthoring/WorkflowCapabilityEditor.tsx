import type { OtpPackageDto, WorkflowCapabilityNodeDto } from '../../application/workflowAuthoring';

type Field = {
  type?: string;
  title?: string;
  minimum?: number;
  maximum?: number;
  enum?: readonly string[];
  properties?: Readonly<Record<string, Field>>;
};

function Fields({
  schema,
  value,
  onChange,
}: {
  schema: Field;
  value: Readonly<Record<string, unknown>>;
  onChange(value: Readonly<Record<string, unknown>>): void;
}) {
  return (
    <>
      {Object.entries(schema.properties ?? {}).map(([key, field]) => {
        const label = field.title ?? key.replace(/([A-Z])/g, ' $1');
        const update = (next: unknown) => onChange({ ...value, [key]: next });
        if (field.type === 'object')
          return (
            <fieldset key={key}>
              <legend>{label}</legend>
              <Fields
                schema={field}
                value={(value[key] as Readonly<Record<string, unknown>>) ?? {}}
                onChange={update}
              />
            </fieldset>
          );
        return (
          <label key={key}>
            {label}
            {field.type === 'boolean' ? (
              <input
                type="checkbox"
                checked={value[key] === true}
                onChange={(event) => update(event.currentTarget.checked)}
              />
            ) : field.enum ? (
              <select
                value={String(value[key] ?? '')}
                onChange={(event) => update(event.currentTarget.value)}
              >
                <option value="">Choose…</option>
                {field.enum.map((option) => (
                  <option key={option}>{option}</option>
                ))}
              </select>
            ) : field.type === 'integer' || field.type === 'number' ? (
              <input
                type="number"
                min={field.minimum}
                max={field.maximum}
                step={field.type === 'integer' ? 1 : 'any'}
                value={typeof value[key] === 'number' ? value[key] : ''}
                onChange={(event) => {
                  if (Number.isFinite(event.currentTarget.valueAsNumber))
                    update(event.currentTarget.valueAsNumber);
                }}
              />
            ) : (
              <textarea
                rows={3}
                value={String(value[key] ?? '')}
                onChange={(event) => update(event.currentTarget.value)}
              />
            )}
          </label>
        );
      })}
    </>
  );
}

export function WorkflowCapabilityEditor({
  node,
  packages,
  onChange,
}: {
  node: WorkflowCapabilityNodeDto;
  packages: readonly OtpPackageDto[];
  onChange(node: WorkflowCapabilityNodeDto): void;
}) {
  const descriptor = packages
    .find((pkg) => pkg.id === node.capability.package)
    ?.nodeCapabilities?.find((item) => item.id === node.capability.tool);
  return (
    <div className="workflow-connection-editor__grid">
      <label>
        Name
        <input
          value={node.name}
          onChange={(event) => onChange({ ...node, name: event.currentTarget.value })}
        />
      </label>
      <p>
        {descriptor?.name ?? node.capability.tool} · {node.capability.package}
      </p>
      {descriptor ? (
        <Fields
          schema={descriptor.configurationSchema}
          value={node.configuration}
          onChange={(configuration) => onChange({ ...node, configuration })}
        />
      ) : (
        <p role="alert">This capability is unavailable on this device.</p>
      )}
    </div>
  );
}
