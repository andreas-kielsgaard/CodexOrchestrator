import { useState } from 'react';
import type { OtpPackageDto } from '../../application/otp';
import type { CatalogState } from '../../components/CatalogSelect';
import { OtpElementDetails } from '../otp/otpElements';
import { mcpToolCatalogValue } from './types';
import { GroupedCapabilityPicker, type CapabilityPickerNode } from './GroupedCapabilityPicker';

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
  const optionById = new Map(catalog.options.map((option) => [option.value, option]));
  const groups: CapabilityPickerNode[] = [];
  for (const pkg of packages) {
    const children: CapabilityPickerNode[] = [];
    const workflowItems = pkg.tools
      .filter((tool) => tool.entrypoint.kind === 'mcp')
      .map((tool) => optionById.get(mcpToolCatalogValue(pkg.id, tool.id)))
      .filter((option): option is NonNullable<typeof option> => Boolean(option));
    if (workflowItems.length) {
      children.push({ id: `otp:${pkg.id}:workflow`, label: 'Workflow tools', items: workflowItems });
    }
    for (const server of pkg.agentMcpServers ?? []) {
      const capabilityGroups = server.capabilityGroups
        .map((group) => ({
          id: `otp:${pkg.id}:${server.serverName}:${group.id}`,
          label: group.name,
          items: server.tools
            .filter((tool) => tool.capability === group.id)
            .map((tool) => optionById.get(mcpToolCatalogValue(server.serverName, tool.id)))
            .filter((option): option is NonNullable<typeof option> => Boolean(option)),
        }))
        .filter((group) => group.items.length);
      if (capabilityGroups.length) {
        children.push({
          id: `otp:${pkg.id}:${server.serverName}`,
          label: server.name,
          children: capabilityGroups,
        });
      }
    }
    if (children.length) groups.push({ id: `otp:${pkg.id}`, label: pkg.name, children });
  }
  const managedIds = new Set(managed.keys());
  const external = new Map<string, typeof catalog.options>();
  for (const option of catalog.options) {
    if (managedIds.has(option.value)) continue;
    const server = option.value.split('\u0000')[0];
    external.set(server, [...(external.get(server) ?? []), option]);
  }
  groups.push(
    ...[...external].map(([server, items]) => ({
      id: `mcp:${server}`,
      label: `MCP server: ${server}`,
      items,
    })),
  );
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
        <GroupedCapabilityPicker
          title="Set MCP tools"
          groups={groups}
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
