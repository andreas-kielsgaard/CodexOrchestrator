import type { ExecutionBindingDto } from '../executionTargets/contracts';
export type SandboxModeDto = 'read_only' | 'workspace_write' | 'danger_full_access';
export type CodexPersonalityDto = 'none' | 'friendly' | 'pragmatic';

/** Capabilities exposed or permitted at one execution-configuration boundary. */
export interface CapabilitySetDto {
  readonly models: readonly string[];
  readonly reasoningModes: readonly string[];
  readonly sandboxModes: readonly SandboxModeDto[];
  readonly mcpTools: Readonly<Record<string, readonly string[]>>;
  readonly skills: readonly string[];
}

export interface RuntimeSelectionsDto {
  readonly model: string | null;
  readonly reasoningMode: string | null;
  readonly sandboxMode: SandboxModeDto | null;
}

/** A model and the inclusive reasoning range the profile permits on one route. */
export interface ModelAllowanceDto {
  readonly modelId: string;
  readonly minimumReasoning: string;
  readonly maximumReasoning: string;
}

/** One permitted Device -> Harness -> Inference Source route in a Capability Profile. */
export interface ProfileRoutePolicyDto {
  readonly routeId: string;
  readonly execution: ExecutionBindingDto;
  readonly codexPersonality?: CodexPersonalityDto | null;
  readonly modelAllowances: readonly ModelAllowanceDto[];
  readonly mcpGroups: readonly string[];
  readonly skillGroups: readonly string[];
  readonly defaults: RuntimeSelectionsDto;
}

/** Read-only facts observed from the profile's configured device runtime. */
export interface RuntimeProfileSnapshotDto {
  readonly contractVersion: 1;
  readonly profileRef: string;
  readonly exposure: CapabilitySetDto;
  readonly locked: RuntimeSelectionsDto;
  readonly codexPersonality?: CodexPersonalityDto | null;
}

export interface ProfileModelCatalogueDto {
  readonly configurationRef: string;
  readonly observedAt: string | null;
  readonly observationError: string | null;
  readonly models: readonly {
    readonly id: string;
    readonly label: string;
    readonly description: string;
    readonly defaultReasoningMode: string | null;
    readonly reasoningModes: readonly { readonly id: string; readonly description: string }[];
  }[];
}

/** Reusable capability ceiling selected by a Workflow node. */
export interface CapabilityProfileDto {
  readonly execution?: ExecutionBindingDto;
  readonly defaults?: RuntimeSelectionsDto;
  readonly contractVersion: 1;
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly revision: number;
  readonly allowedCapabilities: CapabilitySetDto;
  readonly routePolicies?: readonly ProfileRoutePolicyDto[];
  readonly defaultRouteId?: string | null;
}

/** Workflow-owned configuration embedded in one node. */
export interface NodeProfileDto {
  readonly contractVersion: 1;
  readonly allowedCapabilities: CapabilitySetDto;
  readonly pinnedDefaults: RuntimeSelectionsDto;
}

/** Immutable configuration resolved and pinned when a Session is created. */
export interface SessionProfileDto {
  readonly contractVersion: 1;
  readonly runtimeProfileRef: string;
  readonly attachedRuntimeCapabilities: CapabilitySetDto;
  readonly attachedRuntimeLocked: RuntimeSelectionsDto;
  readonly capabilityProfileId: string;
  readonly capabilityProfileRevision: number;
  readonly nodeCapabilities: CapabilitySetDto;
  readonly pinnedDefaults: RuntimeSelectionsDto;
  readonly codexPersonality?: CodexPersonalityDto | null;
}

export interface SessionCreationResolutionDto {
  readonly contractVersion: 1;
  readonly sessionProfile: SessionProfileDto;
  readonly digest: string;
}

export interface DirectUserInvocationResolutionDto {
  readonly contractVersion: 1;
  readonly sessionProfileDigest: string;
  readonly selections: RuntimeSelectionsDto;
}

export interface CreateCapabilityProfileInput {
  readonly execution?: ExecutionBindingDto;
  readonly defaults?: RuntimeSelectionsDto;
  readonly name: string;
  readonly allowedCapabilities: CapabilitySetDto;
  readonly routePolicies?: readonly ProfileRoutePolicyDto[];
  readonly defaultRouteId?: string | null;
}

export interface UpdateCapabilityProfileInput {
  readonly execution?: ExecutionBindingDto;
  readonly defaults?: RuntimeSelectionsDto;
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly allowedCapabilities: CapabilitySetDto;
  readonly routePolicies?: readonly ProfileRoutePolicyDto[];
  readonly defaultRouteId?: string | null;
}

export interface ExecutionConfigurationClient {
  loadProfileModelCatalogue?(configurationRef: string): Promise<ProfileModelCatalogueDto>;
  loadDefaultCapabilityProfile?(): Promise<string | null>;
  setDefaultCapabilityProfile?(capabilityProfileId: string): Promise<void>;
  loadSelectedRuntimeProfile(): Promise<RuntimeProfileSnapshotDto>;
  listCapabilityProfiles(): Promise<readonly CapabilityProfileDto[]>;
  loadCapabilityProfile(capabilityProfileId: string): Promise<CapabilityProfileDto>;
  createCapabilityProfile(input: CreateCapabilityProfileInput): Promise<CapabilityProfileDto>;
  updateCapabilityProfile(input: UpdateCapabilityProfileInput): Promise<CapabilityProfileDto>;
  deleteCapabilityProfile(capabilityProfileId: string): Promise<void>;
}
