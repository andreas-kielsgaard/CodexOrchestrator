import type { ReactNode } from 'react';
import type {
  OtpAgentMcpCapabilityGroupDto,
  OtpAgentMcpServerDto,
  OtpAgentMcpToolDto,
  OtpOutputDto,
  OtpToolDto,
} from '../../application/otp';

export function OtpElementDetails({
  tool,
  output,
  server,
  group,
  endpoint,
}: {
  readonly tool?: OtpToolDto;
  readonly output?: OtpOutputDto;
  readonly server?: OtpAgentMcpServerDto;
  readonly group?: OtpAgentMcpCapabilityGroupDto;
  readonly endpoint?: OtpAgentMcpToolDto;
}) {
  if (endpoint && server) return <EndpointDetails endpoint={endpoint} server={server} />;
  if (group && server) return <GroupDetails group={group} server={server} />;
  if (server) return <ServerDetails server={server} />;
  if (tool) return <WorkflowToolDetails tool={tool} output={output} />;
  return <p>Select an offered element to inspect its details.</p>;
}

function WorkflowToolDetails({ tool, output }: { readonly tool: OtpToolDto; readonly output?: OtpOutputDto }) {
  const type = tool.entrypoint.kind === 'action' ? 'Workflow destination' : 'Workflow trigger or event source';
  return (
    <div className="otp-element-details">
      <p className="otp-element-details__type">{type}</p>
      <h3>{output ? `${tool.name} · ${output.name}` : tool.name}</h3>
      <DetailSection title="Purpose"><p>{tool.description}</p></DetailSection>
      <DetailSection title="Expected behavior"><p>{tool.expectedBehavior}</p></DetailSection>
      <DetailSection title="Recommended use"><p>{tool.recommendedUsage}</p></DetailSection>
      <DetailSection title="Integration">
        <p>{tool.entrypoint.kind === 'action'
          ? 'Available to Workflow connections and initial entry as a destination.'
          : 'Can offer data to connections configured for this exact Workflow source and output.'}</p>
      </DetailSection>
      {tool.entrypoint.kind === 'mcp' ? <SchemaSection title="Input fields" schema={tool.entrypoint.inputSchema} /> : null}
      {output ? <SchemaSection title="Offered output" schema={output.schema} /> : null}
      {!output && tool.outputs.length > 0 ? (
        <DetailSection title="Declared results">
          <ul>{tool.outputs.map((item) => <li key={item.id}>{item.name}</li>)}</ul>
        </DetailSection>
      ) : null}
      {tool.configuration.length ? (
        <DetailSection title="Configuration">
          <ul>{tool.configuration.map((field) => <li key={field.key}>{field.label}{field.choices.length ? `: ${field.choices.join(', ').replaceAll('_', ' ')}` : ''}</li>)}</ul>
        </DetailSection>
      ) : null}
    </div>
  );
}

function ServerDetails({ server }: { readonly server: OtpAgentMcpServerDto }) {
  return (
    <div className="otp-element-details">
      <p className="otp-element-details__type">MCP service</p>
      <h3>{server.name}</h3>
      <DetailSection title="Purpose"><p>{server.description}</p></DetailSection>
      <DetailSection title="Integration"><p>Available through configured agent sessions. It is not a direct Workflow connection trigger or destination.</p></DetailSection>
      <DetailSection title="Offers"><p>{server.capabilityGroups.length} capability groups and {server.tools.length} endpoints.</p></DetailSection>
    </div>
  );
}

function GroupDetails({ group, server }: { readonly group: OtpAgentMcpCapabilityGroupDto; readonly server: OtpAgentMcpServerDto }) {
  const endpoints = server.tools.filter((tool) => tool.capability === group.id);
  return (
    <div className="otp-element-details">
      <p className="otp-element-details__type">MCP capability group</p>
      <h3>{group.name}</h3>
      <DetailSection title="Purpose"><p>{group.description}</p></DetailSection>
      <DetailSection title="Integration"><p>Available through configured agent sessions using {server.name}.</p></DetailSection>
      <DetailSection title="Endpoints"><ul>{endpoints.map((tool) => <li key={tool.id}>{tool.name}</li>)}</ul></DetailSection>
    </div>
  );
}

function EndpointDetails({ endpoint, server }: { readonly endpoint: OtpAgentMcpToolDto; readonly server: OtpAgentMcpServerDto }) {
  const grants = endpoint.requiredGrants
    .map((id) => server.grants.find((grant) => grant.id === id))
    .filter((grant): grant is NonNullable<typeof grant> => Boolean(grant));
  return (
    <div className="otp-element-details">
      <p className="otp-element-details__type">MCP endpoint</p>
      <h3>{endpoint.name}</h3>
      <DetailSection title="Purpose"><p>{endpoint.description}</p></DetailSection>
      <DetailSection title="Expected behavior"><p>{endpoint.expectedBehavior}</p></DetailSection>
      <DetailSection title="Recommended use"><p>{endpoint.recommendedUsage}</p></DetailSection>
      <SchemaSection title="Input fields" schema={endpoint.inputSchema} />
      <SchemaSection title="Successful response" schema={endpoint.outputSchema} />
      <DetailSection title="Required authorization">
        {grants.length ? <ul>{grants.map((grant) => <li key={grant.id}><strong>{grant.label}</strong><br />{grant.description}</li>)}</ul> : <p>No mutation grant is required.</p>}
      </DetailSection>
      <DetailSection title="Integration"><p>Available through configured agent sessions using {server.name}. It is not a direct Workflow connection trigger or destination.</p></DetailSection>
    </div>
  );
}

function DetailSection({ title, children }: { readonly title: string; readonly children: ReactNode }) {
  return <section><h4>{title}</h4>{children}</section>;
}

function SchemaSection({ title, schema }: { readonly title: string; readonly schema: Readonly<Record<string, unknown>> }) {
  const properties = schema.properties && typeof schema.properties === 'object'
    ? Object.entries(schema.properties as Record<string, unknown>)
    : [];
  const required = new Set(Array.isArray(schema.required) ? schema.required.filter((value): value is string => typeof value === 'string') : []);
  return (
    <DetailSection title={title}>
      {properties.length ? (
        <dl className="otp-element-details__schema">
          {properties.map(([name, definition]) => {
            const type = definition && typeof definition === 'object' && 'type' in definition
              ? String((definition as { readonly type?: unknown }).type ?? 'value')
              : 'value';
            return <div key={name}><dt>{name}</dt><dd>{type}{required.has(name) ? ' · required' : ''}</dd></div>;
          })}
        </dl>
      ) : <p>No structured fields are declared.</p>}
    </DetailSection>
  );
}
