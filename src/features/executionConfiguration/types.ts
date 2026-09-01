import type { PresentableIdentity } from '../identities';
import type { CatalogState } from '../../components/CatalogSelect';
import type {
  CapabilitySetDto,
  RuntimeSelectionsDto,
  SandboxModeDto,
} from '../../application/executionConfiguration';

/** The capability contract is already browser-safe; editors consume it directly. */
export type CapabilitySetViewModel = CapabilitySetDto;

export type RuntimeSelectionsViewModel = RuntimeSelectionsDto;

export interface RuntimeCapabilityCatalogs {
  readonly models: CatalogState;
  readonly reasoningModes: CatalogState;
  readonly sandboxModes: CatalogState<SandboxModeDto>;
  readonly mcpTools: CatalogState;
  readonly skills: CatalogState;
}

export interface RuntimeProfileViewModel {
  readonly profileRef: string;
  readonly sourceLabel: string;
  readonly exposure: CapabilitySetViewModel;
  readonly catalogs: RuntimeCapabilityCatalogs;
  readonly lockedSelections: RuntimeSelectionsViewModel;
  readonly notes?: readonly string[];
}

export interface CapabilityProfileDraft {
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly revision: number | null;
  readonly allowedCapabilities: CapabilitySetViewModel;
}

export interface CapabilityProfileOption {
  readonly id: string;
  readonly label: string;
  readonly revision: number;
}

export interface AgentIdentityOption extends PresentableIdentity {
  readonly id: string;
}

export interface NodeProfileEditorValue {
  readonly nodeName: string;
  readonly identityId: string | null;
  readonly capabilityProfileId: string | null;
  readonly initialPrompt: string;
  readonly exposedCapabilities: CapabilitySetViewModel;
  readonly pinnedDefaults: RuntimeSelectionsViewModel;
}

export interface NodeProfileCopySource {
  readonly nodeId: string;
  readonly nodeName: string;
  readonly summary?: string;
}

export interface SessionProfileViewModel {
  readonly runtimeProfileRef: string;
  readonly capabilityProfileId: string;
  readonly capabilityProfileRevision: number;
  readonly attachedRuntimeCapabilities: CapabilitySetViewModel;
  readonly attachedRuntimeLocked: RuntimeSelectionsViewModel;
  readonly nodeCapabilities: CapabilitySetViewModel;
  readonly pinnedDefaults: RuntimeSelectionsViewModel;
  readonly resolutionDigest?: string;
}

const MCP_TOOL_SEPARATOR = '\u0000';

/** Stable form-control value for one MCP connection/tool pair. */
export function mcpToolCatalogValue(connectionId: string, toolId: string): string {
  return `${connectionId}${MCP_TOOL_SEPARATOR}${toolId}`;
}

export function selectedMcpToolValues(mcpTools: CapabilitySetDto['mcpTools']): readonly string[] {
  return Object.entries(mcpTools).flatMap(([connectionId, toolIds]) =>
    toolIds.map((toolId) => mcpToolCatalogValue(connectionId, toolId)),
  );
}

export function mcpToolsFromSelectedValues(
  values: readonly string[],
): CapabilitySetDto['mcpTools'] {
  const grouped: Record<string, string[]> = {};
  for (const value of values) {
    const [connectionId, toolId] = value.split(MCP_TOOL_SEPARATOR);
    if (!connectionId || !toolId) continue;
    grouped[connectionId] = [...(grouped[connectionId] ?? []), toolId];
  }
  return grouped;
}

export function describeMcpTools(mcpTools: CapabilitySetDto['mcpTools']): readonly string[] {
  return Object.entries(mcpTools).flatMap(([connectionId, toolIds]) =>
    toolIds.map((toolId) => `${connectionId}/${toolId}`),
  );
}
