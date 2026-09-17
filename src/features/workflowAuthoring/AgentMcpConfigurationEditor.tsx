import type { OtpAgentMcpServerDto, OtpPackageDto } from '../../application/otp';
import { OtpConfigurationEditor } from './OtpConfigurationEditor';

type AgentServerConfiguration = Readonly<Record<string, unknown>>;
export type AgentMcpConfiguration = Readonly<Record<string, Readonly<Record<string, AgentServerConfiguration>>>>;

export function AgentMcpConfigurationEditor({
  packages,
  selectedTools,
  value,
  onChange,
}: {
  readonly packages: readonly OtpPackageDto[];
  readonly selectedTools: Readonly<Record<string, readonly string[]>>;
  readonly value: AgentMcpConfiguration;
  readonly onChange: (value: AgentMcpConfiguration) => void;
}) {
  const configured = packages.flatMap((pkg) =>
    pkg.agentMcpServers
      .filter((server) => (selectedTools[server.serverName] ?? []).length > 0)
      .map((server) => ({ pkg, server })),
  );
  if (!configured.length) return null;

  const update = (
    packageId: string,
    server: OtpAgentMcpServerDto,
    next: Readonly<Record<string, unknown>>,
  ) => {
    onChange({
      ...value,
      [packageId]: {
        ...(value[packageId] ?? {}),
        [server.serverName]: next,
      },
    });
  };

  return (
    <section className="workflow-node-otp-configuration">
      <h3>OTP MCP configuration</h3>
      <p>These grants apply only to Sessions created for this node.</p>
      {configured.map(({ pkg, server }) => (
        <article key={`${pkg.id}/${server.serverName}`}>
          <h4>{server.name}</h4>
          <p>{server.description}</p>
          <OtpConfigurationEditor
            fields={server.configuration}
            value={value[pkg.id]?.[server.serverName] ?? {}}
            onChange={(next) => update(pkg.id, server, next)}
          />
        </article>
      ))}
    </section>
  );
}

