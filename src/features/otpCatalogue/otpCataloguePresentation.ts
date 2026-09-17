import type {
  OtpAgentMcpCapabilityGroupDto,
  OtpAgentMcpServerDto,
  OtpAgentMcpToolDto,
  OtpPackageDto,
  OtpToolDto,
} from '../../application/otp';

export type OtpCatalogueElement =
  | {
      readonly key: string;
      readonly type: 'workflow-source' | 'workflow-destination';
      readonly tool: OtpToolDto;
    }
  | { readonly key: string; readonly type: 'mcp-service'; readonly server: OtpAgentMcpServerDto }
  | {
      readonly key: string;
      readonly type: 'mcp-capability-group';
      readonly server: OtpAgentMcpServerDto;
      readonly group: OtpAgentMcpCapabilityGroupDto;
    }
  | {
      readonly key: string;
      readonly type: 'mcp-endpoint';
      readonly server: OtpAgentMcpServerDto;
      readonly endpoint: OtpAgentMcpToolDto;
    };

export interface OtpCatalogueElementSection {
  readonly key: string;
  readonly label: string;
  readonly elements: readonly OtpCatalogueElementListItem[];
}

export interface OtpCatalogueElementListItem {
  readonly element: OtpCatalogueElement;
  readonly depth: 0 | 1 | 2;
}

export function packageInventory(pkg: OtpPackageDto) {
  const workflowSources = pkg.tools.filter((tool) => tool.entrypoint.kind !== 'action').length;
  const workflowDestinations = pkg.tools.filter((tool) => tool.entrypoint.kind === 'action').length;
  const mcpServices = pkg.agentMcpServers.length;
  const capabilityGroups = pkg.agentMcpServers.reduce(
    (sum, server) => sum + server.capabilityGroups.length,
    0,
  );
  const endpoints = pkg.agentMcpServers.reduce((sum, server) => sum + server.tools.length, 0);
  return { workflowSources, workflowDestinations, mcpServices, capabilityGroups, endpoints };
}

export function workflowSources(pkg: OtpPackageDto): OtpCatalogueElement[] {
  return pkg.tools
    .filter((tool) => tool.entrypoint.kind !== 'action')
    .map((tool) => ({ key: `${pkg.id}:tool:${tool.id}`, type: 'workflow-source', tool }));
}

export function workflowDestinations(pkg: OtpPackageDto): OtpCatalogueElement[] {
  return pkg.tools
    .filter((tool) => tool.entrypoint.kind === 'action')
    .map((tool) => ({ key: `${pkg.id}:tool:${tool.id}`, type: 'workflow-destination', tool }));
}

export function packageElementSections(pkg: OtpPackageDto): readonly OtpCatalogueElementSection[] {
  const sections: OtpCatalogueElementSection[] = [];
  const sources = workflowSources(pkg);
  if (sources.length) {
    sections.push({
      key: 'workflow-sources',
      label: 'Trigger and event sources',
      elements: sources.map((element) => ({ element, depth: 0 })),
    });
  }
  const destinations = workflowDestinations(pkg);
  if (destinations.length) {
    sections.push({
      key: 'workflow-destinations',
      label: 'Destinations',
      elements: destinations.map((element) => ({ element, depth: 0 })),
    });
  }
  const mcpElements = pkg.agentMcpServers.flatMap((server) => [
    { element: serverElement(pkg, server), depth: 0 as const },
    ...server.capabilityGroups.flatMap((group) => [
      { element: groupElement(pkg, server, group), depth: 1 as const },
      ...server.tools
        .filter((endpoint) => endpoint.capability === group.id)
        .map((endpoint) => ({
          element: endpointElement(pkg, server, endpoint),
          depth: 2 as const,
        })),
    ]),
  ]);
  if (mcpElements.length) {
    sections.push({ key: 'mcp-services', label: 'MCP services', elements: mcpElements });
  }
  return sections;
}

export function serverElement(
  pkg: OtpPackageDto,
  server: OtpAgentMcpServerDto,
): OtpCatalogueElement {
  return { key: `${pkg.id}:server:${server.serverName}`, type: 'mcp-service', server };
}

export function groupElement(
  pkg: OtpPackageDto,
  server: OtpAgentMcpServerDto,
  group: OtpAgentMcpCapabilityGroupDto,
): OtpCatalogueElement {
  return {
    key: `${pkg.id}:server:${server.serverName}:group:${group.id}`,
    type: 'mcp-capability-group',
    server,
    group,
  };
}

export function endpointElement(
  pkg: OtpPackageDto,
  server: OtpAgentMcpServerDto,
  endpoint: OtpAgentMcpToolDto,
): OtpCatalogueElement {
  return {
    key: `${pkg.id}:server:${server.serverName}:endpoint:${endpoint.id}`,
    type: 'mcp-endpoint',
    server,
    endpoint,
  };
}

export function elementLabel(element: OtpCatalogueElement) {
  if ('tool' in element) return element.tool.name;
  if ('group' in element) return element.group.name;
  if ('endpoint' in element) return element.endpoint.name;
  return element.server.name;
}

export function elementTypeLabel(element: OtpCatalogueElement) {
  switch (element.type) {
    case 'workflow-source':
      return 'Trigger or event source';
    case 'workflow-destination':
      return 'Destination';
    case 'mcp-service':
      return 'MCP service';
    case 'mcp-capability-group':
      return 'Capability group';
    case 'mcp-endpoint':
      return 'MCP endpoint';
  }
}
