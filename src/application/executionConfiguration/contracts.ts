export type SandboxModeDto = 'read_only' | 'workspace_write' | 'danger_full_access';

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

/** Read-only facts observed from the currently selected native runtime. */
export interface RuntimeProfileSnapshotDto {
  readonly contractVersion: 1;
  readonly profileRef: string;
  readonly exposure: CapabilitySetDto;
  readonly locked: RuntimeSelectionsDto;
}

/** Reusable capability ceiling selected by a Workflow node. */
export interface CapabilityProfileDto {
  readonly defaults?: RuntimeSelectionsDto;
  readonly contractVersion: 1;
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly revision: number;
  readonly allowedCapabilities: CapabilitySetDto;
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
  readonly defaults?: RuntimeSelectionsDto;
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly allowedCapabilities: CapabilitySetDto;
}

export interface UpdateCapabilityProfileInput {
  readonly defaults?: RuntimeSelectionsDto;
  readonly capabilityProfileId: string;
  readonly name: string;
  readonly allowedCapabilities: CapabilitySetDto;
}

export interface ExecutionConfigurationClient {
  loadNativeCapabilityInventory?(): Promise<NativeCapabilityInventoryDto>;
  loadDefaultCapabilityProfile?(): Promise<string | null>;
  setDefaultCapabilityProfile?(capabilityProfileId: string): Promise<void>;
  loadSelectedRuntimeProfile(): Promise<RuntimeProfileSnapshotDto>;
  listCapabilityProfiles(): Promise<readonly CapabilityProfileDto[]>;
  loadCapabilityProfile(capabilityProfileId: string): Promise<CapabilityProfileDto>;
  createCapabilityProfile(input: CreateCapabilityProfileInput): Promise<CapabilityProfileDto>;
  updateCapabilityProfile(input: UpdateCapabilityProfileInput): Promise<CapabilityProfileDto>;
  deleteCapabilityProfile(capabilityProfileId: string): Promise<void>;
}

export interface NativeCapabilityInventoryDto {
  readonly entries: readonly {
    readonly name: string;
    readonly kind: string;
    readonly origin: string;
    readonly state: string;
    readonly support: string;
  }[];
  readonly limitations: readonly string[];
}
