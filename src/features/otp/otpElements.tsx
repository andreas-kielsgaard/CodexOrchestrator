import type { OtpToolDto, OtpOutputDto } from '../../application/otp';
export function OtpElementDetails({
  tool,
  output,
}: {
  readonly tool: OtpToolDto;
  readonly output?: OtpOutputDto;
}) {
  return (
    <>
      <h3>
        {tool.name}
        {output ? ` · ${output.name}` : ''}
      </h3>
      <p>{tool.description}</p>
      <dl>
        <dt>Tool</dt>
        <dd>{tool.id}</dd>
        {output && (
          <>
            <dt>Output</dt>
            <dd>{output.id}</dd>
            <dt>Offered fields</dt>
            <dd>
              <pre>{JSON.stringify(output.schema, null, 2)}</pre>
            </dd>
          </>
        )}
        {tool.entrypoint.kind === 'mcp' && (
          <>
            <dt>Arguments</dt>
            <dd>
              <pre>{JSON.stringify(tool.entrypoint.inputSchema, null, 2)}</pre>
            </dd>
          </>
        )}
        {tool.configuration.length > 0 && (
          <>
            <dt>Configuration</dt>
            <dd>
              {tool.configuration.map((field) => (
                <p key={field.key}>
                  {field.label}
                  {field.choices.length ? `: ${field.choices.join(', ').replaceAll('_', ' ')}` : ''}
                </p>
              ))}
            </dd>
          </>
        )}
      </dl>
    </>
  );
}
