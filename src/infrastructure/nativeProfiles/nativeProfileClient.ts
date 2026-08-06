import { invoke } from '@tauri-apps/api/core';

export type NativeProfileOwnership = 'registered_existing' | 'application_dedicated';
export type NativeProfileLifecycle =
  | 'active'
  | 'missing_or_moved'
  | 'replaced'
  | 'foreign'
  | 'malformed';
export type NativeProfileAuthentication = 'unknown' | 'authenticated' | 'unauthenticated';
export type NativeProfileSandbox = 'unknown' | 'initialized' | 'failed' | 'attention_required';
export type NativeProfileCanary = 'not_run' | 'passed' | 'blocked';
export type NativeProfileMcp = 'not_assessed' | 'ready' | 'probe_failed';
export type NativeExecutionMode = 'workspace_write' | 'danger_full_access';

export interface NativeProfileAttentions {
  readonly authentication: string | null;
  readonly sandbox: string | null;
  readonly canary: string | null;
  readonly mcpReporting: string | null;
  readonly continuity: string | null;
  readonly cli: string | null;
}

export interface NativeProfileReadiness {
  readonly authentication: NativeProfileAuthentication;
  readonly sandboxInitialization: NativeProfileSandbox;
  readonly workspaceWriteCanary: NativeProfileCanary;
  readonly dangerFullAccessCanary: NativeProfileCanary;
  readonly mcpReporting: NativeProfileMcp;
  readonly attentions: NativeProfileAttentions;
}

export interface NativeProfileExecution {
  readonly selectedMode: NativeExecutionMode;
  readonly dangerFullAccessAuthorized: boolean;
}

export interface NativeProfile {
  readonly id: string;
  readonly homePath: string;
  readonly ownership: NativeProfileOwnership;
  readonly lifecycle: NativeProfileLifecycle;
  readonly selected: boolean;
  readonly execution: NativeProfileExecution;
  readonly readiness: NativeProfileReadiness;
}

export interface NativeProfileQuery {
  readonly contract: 'native-codex-profile-query/v1';
  readonly profiles: readonly NativeProfile[];
}

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
const noActionArgs = undefined;

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function keys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  for (const key of Object.keys(value))
    if (!allowed.includes(key)) throw new Error(`${label} contains unknown field: ${key}`);
}
function stringValue(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.length === 0 || value.trim() !== value)
    throw new Error(`${label} must be a non-empty trimmed string`);
  return value;
}
function enumValue<T extends string>(value: unknown, values: readonly T[], label: string): T {
  if (typeof value !== 'string' || !values.includes(value as T)) throw new Error(`${label} is invalid`);
  return value as T;
}
function nullableString(value: unknown, label: string): string | null {
  if (value !== null && (typeof value !== 'string' || value.length === 0 || value.trim() !== value))
    throw new Error(`${label} must be null or a non-empty trimmed string`);
  return value as string | null;
}

function absolutePath(value: unknown, label: string): string {
  const path = stringValue(value, label);
  if (!/^(?:[A-Za-z]:[\\/]|\\\\[^\\/]+[\\/][^\\/]+|\/)/.test(path))
    throw new Error(`${label} must be absolute`);
  return path;
}

export function decodeNativeProfileQuery(value: unknown): NativeProfileQuery {
  const query = object(value, 'native profile query');
  keys(query, ['contract', 'profiles'], 'native profile query');
  if (query.contract !== 'native-codex-profile-query/v1') throw new Error('Unsupported native profile contract');
  if (!Array.isArray(query.profiles)) throw new Error('native profile query profiles must be an array');
  const profiles = query.profiles.map((item, index) => decodeNativeProfile(item, index));
  if (new Set(profiles.map((profile) => profile.id)).size !== profiles.length)
    throw new Error('Native profile query contains duplicate profile ids');
  if (profiles.filter((profile) => profile.selected).length > 1)
    throw new Error('Native profile query contains multiple selected profiles');
  if (profiles.some((profile) => profile.selected && profile.lifecycle !== 'active'))
    throw new Error('Native profile query selects a stale or invalid profile');
  return { contract: query.contract, profiles };
}

export function decodeNativeProfile(value: unknown, index = 0): NativeProfile {
  const profile = object(value, `native profile ${index}`);
  keys(profile, ['id', 'homePath', 'ownership', 'lifecycle', 'selected', 'execution', 'readiness'], `native profile ${index}`);
  if (typeof profile.selected !== 'boolean') throw new Error(`native profile ${index} selected must be boolean`);
  const readiness = object(profile.readiness, `native profile ${index} readiness`);
  keys(readiness, ['authentication', 'sandboxInitialization', 'workspaceWriteCanary', 'dangerFullAccessCanary', 'mcpReporting', 'attentions'], 'readiness');
  const execution = object(profile.execution, `native profile ${index} execution`);
  keys(execution, ['selectedMode', 'dangerFullAccessAuthorized'], `native profile ${index} execution`);
  if (typeof execution.dangerFullAccessAuthorized !== 'boolean') throw new Error(`native profile ${index} danger authorization must be boolean`);
  const attentions = object(readiness.attentions, 'attentions');
  keys(attentions, ['authentication', 'sandbox', 'canary', 'mcpReporting', 'continuity', 'cli'], 'attentions');
  return {
    id: stringValue(profile.id, 'profile id'),
    homePath: absolutePath(profile.homePath, 'profile home path'),
    ownership: enumValue(profile.ownership, ['registered_existing', 'application_dedicated'], 'profile ownership'),
    lifecycle: enumValue(profile.lifecycle, ['active', 'missing_or_moved', 'replaced', 'foreign', 'malformed'], 'profile lifecycle'),
    selected: profile.selected,
    execution: {
      selectedMode: enumValue(execution.selectedMode, ['workspace_write', 'danger_full_access'], 'execution mode'),
      dangerFullAccessAuthorized: execution.dangerFullAccessAuthorized,
    },
    readiness: {
      authentication: enumValue(readiness.authentication, ['unknown', 'authenticated', 'unauthenticated'], 'authentication'),
      sandboxInitialization: enumValue(readiness.sandboxInitialization, ['unknown', 'initialized', 'failed', 'attention_required'], 'sandbox initialization'),
      workspaceWriteCanary: enumValue(readiness.workspaceWriteCanary, ['not_run', 'passed', 'blocked'], 'workspace canary'),
      dangerFullAccessCanary: enumValue(readiness.dangerFullAccessCanary, ['not_run', 'passed', 'blocked'], 'full access canary'),
      mcpReporting: enumValue(readiness.mcpReporting, ['not_assessed', 'ready', 'probe_failed'], 'MCP reporting'),
      attentions: {
        authentication: nullableString(attentions.authentication, 'authentication attention'),
        sandbox: nullableString(attentions.sandbox, 'sandbox attention'),
        canary: nullableString(attentions.canary, 'canary attention'),
        mcpReporting: nullableString(attentions.mcpReporting, 'MCP attention'),
        continuity: nullableString(attentions.continuity, 'continuity attention'),
        cli: nullableString(attentions.cli, 'CLI attention'),
      },
    },
  };
}

export interface NativeProfileClient {
  load(): Promise<NativeProfileQuery>;
  registerExisting(homePath: string): Promise<NativeProfileQuery>;
  createDedicated(): Promise<NativeProfileQuery>;
  select(profileId: string): Promise<NativeProfileQuery>;
  selectExecutionMode(profileId: string, mode: NativeExecutionMode): Promise<NativeProfileQuery>;
  authorizeDangerFullAccess(profileId: string): Promise<NativeProfileQuery>;
  revokeDangerFullAccess(profileId: string): Promise<NativeProfileQuery>;
  requestLogin(profileId: string): Promise<NativeProfileQuery>;
  refreshReadiness(profileId: string): Promise<NativeProfileQuery>;
  initializeSandbox(profileId: string): Promise<NativeProfileQuery>;
  confirmSandboxInitialization(profileId: string): Promise<NativeProfileQuery>;
  runCanary(profileId: string): Promise<NativeProfileQuery>;
  runDangerFullAccessCanary(profileId: string): Promise<NativeProfileQuery>;
  probeMcp(profileId: string): Promise<NativeProfileQuery>;
}

export function createNativeProfileClient(invokeCommand: Invoke = invoke): NativeProfileClient {
  let queue = Promise.resolve();
  const read = () => invokeCommand<unknown>('load_native_profile_query').then(decodeNativeProfileQuery);
  const load = () => {
    const run = queue.then(read);
    queue = run.then(() => undefined, () => undefined);
    return run;
  };
  const action = (command: string, args?: Record<string, unknown>) => {
    const run = queue.then(async () => {
      const actionResult = await invokeCommand<unknown>(command, args);
      decodeNativeProfile(actionResult, 0);
      const result = await read();
      return result;
    });
    queue = run.then(() => undefined, () => undefined);
    return run;
  };
  const id = (profileId: string) => ({ input: { profileId } });
  return {
    load,
    registerExisting: (homePath) => action('register_native_profile', { input: { homePath } }),
    createDedicated: () => action('create_dedicated_native_profile', noActionArgs),
    select: (profileId) => action('select_native_profile', id(profileId)),
    selectExecutionMode: (profileId, mode) => action('select_native_profile_execution_mode', { input: { profileId, mode } }),
    authorizeDangerFullAccess: (profileId) => action('authorize_native_profile_danger_full_access', id(profileId)),
    revokeDangerFullAccess: (profileId) => action('revoke_native_profile_danger_full_access', id(profileId)),
    requestLogin: (profileId) => action('request_native_profile_login', id(profileId)),
    refreshReadiness: (profileId) => action('refresh_native_profile_readiness', id(profileId)),
    initializeSandbox: (profileId) => action('request_native_profile_sandbox_initialization', id(profileId)),
    confirmSandboxInitialization: (profileId) => action('confirm_native_profile_sandbox_initialization', id(profileId)),
    runCanary: (profileId) => action('run_native_profile_workspace_write_canary', id(profileId)),
    runDangerFullAccessCanary: (profileId) => action('run_native_profile_danger_full_access_canary', id(profileId)),
    probeMcp: (profileId) => action('probe_native_profile_mcp_reporting', id(profileId)),
  };
}

export const tauriNativeProfileClient = createNativeProfileClient();
