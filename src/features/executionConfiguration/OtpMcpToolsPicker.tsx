import { useState } from 'react';
import type { OtpPackageDto } from '../../application/otp';
import type { CatalogState } from '../../components/CatalogSelect';
import { OtpElementPicker, type OtpPickerGroup } from '../../components/otp/OtpElementPicker';
import { OtpElementDetails } from '../otp/otpElements';
import { mcpToolCatalogValue } from './types';

type ManagedTool =
  | {
      readonly kind: 'workflow';
      readonly pkg: OtpPackageDto;
      readonly tool: OtpPackageDto['tools'][number];
    }
  | {
      readonly kind: 'agent_mcp';
      readonly pkg: OtpPackageDto;
      readonly server: OtpPackageDto['agentMcpServers'][number];
      readonly serverName: string;
      readonly serverLabel: string;
      readonly tool: OtpPackageDto['agentMcpServers'][number]['tools'][number];
    };

export function OtpMcpToolsPicker({
  packages,
  catalog,
  values,
  disabled,
  onChange,
}: {
  readonly packages: readonly OtpPackageDto[];
  readonly catalog: CatalogState;
  readonly values: readonly string[];
  readonly disabled?: boolean;
  readonly onChange: (values: readonly string[]) => void;
}) {
  const [open, setOpen] = useState(false);
  const managed = new Map<string, ManagedTool>(
    packages.flatMap((pkg) => [
      ...pkg.tools
        .filter((tool) => tool.entrypoint.kind === 'mcp')
        .map(
          (tool) =>
            [
              mcpToolCatalogValue(pkg.id, tool.id),
              { kind: 'workflow', pkg, tool } as const,
            ] as const,
        ),
      ...(pkg.agentMcpServers ?? []).flatMap((server) =>
        server.tools.map(
          (tool) =>
            [
              mcpToolCatalogValue(server.serverName, tool.id),
              {
                kind: 'agent_mcp',
                pkg,
                server,
                serverName: server.serverName,
                serverLabel: server.name,
                tool,
              } as const,
            ] as const,
        ),
      ),
    ]),
  );
  const groups = new Map<
    string,
    { id: string; label: string; items: OtpPickerGroup['items'][number][] }
  >();
  for (const option of catalog.options) {
    const owned = managed.get(option.value);
    const server = option.value.split('\u0000')[0];
    const groupId = owned
      ? `otp:${owned.pkg.id}:${owned.kind === 'agent_mcp' ? owned.serverName : 'workflow'}`
      : `mcp:${server}`;
    const group = groups.get(groupId) ?? {
      id: groupId,
      label:
        owned?.kind === 'agent_mcp'
          ? `${owned.pkg.id}: ${owned.serverLabel}`
          : owned?.pkg.id ?? `MCP server: ${server}`,
      items: [],
    };
    group.items.push({
      id: option.value,
      label: owned?.tool.name ?? option.label,
      disabled: option.disabled,
    });
    groups.set(groupId, group);
  }
  return (
    <div>
      <strong>MCP tools</strong>
      <div className="otp-selection">
        <span className="otp-selection__summary">
          {values.length
            ? values.map((id) => managed.get(id)?.tool.name ?? id.replace('\u0000', '/')).join(', ')
            : 'No tools selected'}
        </span>
        <button
          type="button"
          disabled={disabled || catalog.availability === 'unavailable'}
          onClick={() => setOpen(true)}
        >
          Set MCP tools
        </button>
      </div>
      {catalog.reason && <p>{catalog.reason}</p>}
      {open && (
        <OtpElementPicker
          title="Set MCP tools"
          multiple
          groups={[...groups.values()]}
          selected={values}
          renderDetails={(id) => {
            const owned = managed.get(id);
            if (!owned) {
              return (
                <>
                  <h3>{id.replace('\u0000', '/')}</h3>
                  <p>Tool provided by an MCP server.</p>
                </>
              );
            }
            if (owned.kind === 'workflow') return <OtpElementDetails tool={owned.tool} />;
            return <OtpElementDetails endpoint={owned.tool} server={owned.server} />;
          }}
          onClose={() => setOpen(false)}
          onConfirm={(next) => {
            onChange(next);
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}
