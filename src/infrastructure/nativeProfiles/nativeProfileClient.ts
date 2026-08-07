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
export type NativeDangerAuthorizationDisposition = 'not_authorized' | 'legacy_insufficient' | 'authorized' | 'revoked' | 'foreign';
export type NativeDangerAuthorizationScope = 'filesystem_only' | 'full_machine_filesystem_and_unrestricted_network';
export type NativeFullAccessCanaryDisposition = 'not_requested' | 'pending' | 'passed' | 'launch_failed' | 'terminal_failed' | 'timed_out' | 'cancelled' | 'recovered_unobserved' | 'cleanup_failed' | 'legacy_unverified';
export type NativeProfileLoginDisposition = 'not_requested' | 'pending' | 'launch_failed' | 'terminal_succeeded' | 'terminal_failed' | 'cancelled' | 'recovered_unobserved';
export type NativeProfileBrowserHandoff = 'unobserved';
export type NativeProfileSetupPhase = 'not_requested' | 'sandbox_initialization' | 'workspace_write_canary';
export type NativeProfileSetupDisposition = 'not_requested' | 'pending' | 'launch_failed' | 'terminal_succeeded' | 'terminal_failed' | 'timed_out' | 'cancelled' | 'recovered_unobserved' | 'legacy_unclassified_failed' | 'policy_unsupported';
export type NativeProfileSetupTerminalClassification = 'not_observed' | 'exit_code' | 'receipt_missing' | 'launch_failed' | 'timed_out' | 'cancelled' | 'recovered_unobserved' | 'legacy_unclassified_failed' | 'policy_unsupported';

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
  readonly dangerAuthorization: NativeProfileDangerAuthorization;
}

export interface NativeProfileDangerAuthorization {
  readonly disposition: NativeDangerAuthorizationDisposition;
  readonly authorityScope: NativeDangerAuthorizationScope | null;
  readonly authorityVersion: string | null;
  readonly correlationId: string | null;
  readonly authorizedAt: string | null;
  readonly revokedAt: string | null;
}

export interface NativeProfileLoginAttempt {
  readonly disposition: NativeProfileLoginDisposition;
  readonly browserHandoff: NativeProfileBrowserHandoff;
  readonly requestedAt: string | null;
  readonly launchAcceptedAt: string | null;
  readonly settledAt: string | null;
}

export interface NativeProfileSetupAttempt {
  readonly phase: NativeProfileSetupPhase;
  readonly disposition: NativeProfileSetupDisposition;
  readonly executable: string | null;
  readonly version: string | null;
  readonly workspaceSandboxSupported: boolean | null;
  readonly correlationId: string | null;
  readonly requestedAt: string | null;
  readonly launchAcceptedAt: string | null;
  readonly deadlineAt: string | null;
  readonly settledAt: string | null;
  readonly terminalClassification: NativeProfileSetupTerminalClassification;
  readonly terminalExitCode: number | null;
}

export interface NativeProfileSandboxAdoption {
  readonly disposition: 'not_verified' | 'verified' | 'invalidated';
  readonly executable: string | null;
  readonly version: string | null;
  readonly workspaceSandboxSupported: boolean | null;
  readonly windowsSandboxSetupSupported: boolean | null;
  readonly correlationId: string | null;
  readonly observedAt: string | null;
  readonly elevatedModeObserved: boolean | null;
}

export interface NativeProfileSandboxAdoptionConfirmation {
  readonly disposition: 'not_confirmed' | 'confirmed' | 'invalidated';
  readonly correlationId: string | null;
  readonly confirmedAt: string | null;
}

export interface NativeProfileFullAccessCanaryAttempt {
  readonly disposition: NativeFullAccessCanaryDisposition;
  readonly authorizationVersion: string | null;
  readonly authorizationCorrelationId: string | null;
  readonly correlationId: string | null;
  readonly requestedAt: string | null;
  readonly launchAcceptedAt: string | null;
  readonly deadlineAt: string | null;
  readonly settledAt: string | null;
  readonly processActivity: 'unobserved' | 'launch_accepted' | 'terminal_observed';
  readonly providerActivity: 'unobserved';
  readonly terminalClassification: 'not_observed' | 'exit_code' | 'receipt_missing' | 'launch_failed' | 'timed_out' | 'cancelled' | 'recovered_unobserved' | 'cleanup_failed' | 'legacy_unverified';
  readonly terminalExitCode: number | null;
  readonly receiptObserved: boolean;
  readonly cleanupDisposition: 'pending' | 'removed' | 'failed' | 'not_observed';
}

export interface NativeProfile {
  readonly id: string;
  readonly homePath: string;
  readonly ownership: NativeProfileOwnership;
  readonly lifecycle: NativeProfileLifecycle;
  readonly selected: boolean;
  readonly execution: NativeProfileExecution;
  readonly loginAttempt: NativeProfileLoginAttempt;
  readonly setupAttempt: NativeProfileSetupAttempt;
  readonly sandboxAdoption: NativeProfileSandboxAdoption;
  readonly sandboxAdoptionConfirmation: NativeProfileSandboxAdoptionConfirmation;
  readonly fullAccessCanaryAttempt: NativeProfileFullAccessCanaryAttempt;
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
function booleanValue(value: unknown, label: string): boolean {
  if (typeof value !== 'boolean') throw new Error(`${label} must be boolean`);
  return value;
}
function integerValue(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isInteger(value)) throw new Error(`${label} must be an integer`);
  return value;
}
function nullableString(value: unknown, label: string): string | null {
  if (value !== null && (typeof value !== 'string' || value.length === 0 || value.trim() !== value))
    throw new Error(`${label} must be null or a non-empty trimmed string`);
  return value as string | null;
}
function requiredRfc3339(value: string | null, label: string): string {
  if (value === null || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value) || Number.isNaN(Date.parse(value)))
    throw new Error(`${label} must be an RFC3339 timestamp`);
  return value;
}
function validatePolicyUnsupportedSetupAttempt(attempt: NativeProfileSetupAttempt) {
  if (attempt.disposition !== 'policy_unsupported') return;
  if (!['sandbox_initialization', 'workspace_write_canary'].includes(attempt.phase)
    || attempt.terminalClassification !== 'policy_unsupported'
    || attempt.workspaceSandboxSupported !== false
    || attempt.executable === null
    || attempt.version === null
    || attempt.correlationId === null
    || attempt.launchAcceptedAt !== null
    || attempt.terminalExitCode !== null)
    throw new Error('policy_unsupported setup attempt violates its invariant');
  const requested = requiredRfc3339(attempt.requestedAt, 'policy_unsupported request timestamp');
  const deadline = requiredRfc3339(attempt.deadlineAt, 'policy_unsupported deadline timestamp');
  const settled = requiredRfc3339(attempt.settledAt, 'policy_unsupported settlement timestamp');
  if (Date.parse(deadline) < Date.parse(requested) || Date.parse(settled) < Date.parse(requested))
    throw new Error('policy_unsupported setup attempt has contradictory timestamps');
}

function validateReceiptMissingSetupAttempt(attempt: NativeProfileSetupAttempt) {
  if (attempt.terminalClassification !== 'receipt_missing') return;
  if (attempt.disposition !== 'terminal_failed'
    || attempt.phase !== 'workspace_write_canary'
    || attempt.workspaceSandboxSupported !== true
    || attempt.executable === null
    || attempt.version === null
    || attempt.correlationId === null)
    throw new Error('receipt_missing setup attempt violates its invariant');
  const requested = requiredRfc3339(attempt.requestedAt, 'receipt_missing request timestamp');
  const accepted = requiredRfc3339(attempt.launchAcceptedAt, 'receipt_missing launch timestamp');
  const deadline = requiredRfc3339(attempt.deadlineAt, 'receipt_missing deadline timestamp');
  const settled = requiredRfc3339(attempt.settledAt, 'receipt_missing settlement timestamp');
  if (Date.parse(accepted) < Date.parse(requested) || Date.parse(deadline) < Date.parse(requested) || Date.parse(settled) < Date.parse(accepted))
    throw new Error('receipt_missing setup attempt has contradictory timestamps');
}

function validateDangerAuthorization(authorization: NativeProfileDangerAuthorization, authorized: boolean) {
  const current = authorization.disposition === 'authorized';
  if (authorized !== current) throw new Error('danger authorization boolean contradicts its durable disposition');
  if (authorization.disposition === 'not_authorized') {
    if (authorization.authorityScope !== null || authorization.authorityVersion !== null || authorization.correlationId !== null || authorization.authorizedAt !== null || authorization.revokedAt !== null)
      throw new Error('empty danger authorization has durable facts');
    return;
  }
  if (authorization.authorityScope === null || authorization.authorityVersion === null || authorization.authorizedAt === null)
    throw new Error('danger authorization is missing its durable scope, version, or timestamp');
  requiredRfc3339(authorization.authorizedAt, 'danger authorization timestamp');
  if (authorization.revokedAt !== null) requiredRfc3339(authorization.revokedAt, 'danger authorization revocation timestamp');
  const legacy = authorization.authorityScope === 'filesystem_only' && authorization.authorityVersion === 'danger-full-access/filesystem-only/v1' && authorization.correlationId === null;
  const currentContract = authorization.authorityScope === 'full_machine_filesystem_and_unrestricted_network' && authorization.authorityVersion === 'danger-full-access/unrestricted-network/v1' && authorization.correlationId !== null;
  if (!legacy && !currentContract) throw new Error('danger authorization has an unknown or contradictory authority contract');
  if (authorization.disposition === 'authorized' && (!currentContract || authorization.revokedAt !== null))
    throw new Error('current danger authorization is contradictory');
  if (authorization.disposition === 'legacy_insufficient' && !legacy) throw new Error('legacy danger authorization is contradictory');
  if (authorization.disposition === 'revoked' && authorization.revokedAt === null) throw new Error('revoked danger authorization lacks revocation timestamp');
}

function validateFullAccessCanaryAttempt(attempt: NativeProfileFullAccessCanaryAttempt) {
  if (attempt.disposition === 'not_requested') {
    if (attempt.authorizationVersion !== null || attempt.authorizationCorrelationId !== null || attempt.correlationId !== null || attempt.requestedAt !== null || attempt.launchAcceptedAt !== null || attempt.deadlineAt !== null || attempt.settledAt !== null || attempt.processActivity !== 'unobserved' || attempt.providerActivity !== 'unobserved' || attempt.terminalClassification !== 'not_observed' || attempt.terminalExitCode !== null || attempt.receiptObserved || attempt.cleanupDisposition !== 'not_observed')
      throw new Error('empty full-access canary has durable facts');
    return;
  }
  const requestedAt = requiredRfc3339(attempt.requestedAt, 'full-access canary request timestamp');
  if (attempt.launchAcceptedAt !== null) requiredRfc3339(attempt.launchAcceptedAt, 'full-access canary launch timestamp');
  if (attempt.deadlineAt !== null) requiredRfc3339(attempt.deadlineAt, 'full-access canary deadline timestamp');
  if (attempt.settledAt !== null) requiredRfc3339(attempt.settledAt, 'full-access canary settlement timestamp');
  const requested = Date.parse(requestedAt);
  const launch = attempt.launchAcceptedAt === null ? null : Date.parse(attempt.launchAcceptedAt);
  const deadline = attempt.deadlineAt === null ? null : Date.parse(attempt.deadlineAt);
  const settled = attempt.settledAt === null ? null : Date.parse(attempt.settledAt);
  if ((launch !== null && launch < requested) || (deadline !== null && deadline < requested) || (settled !== null && (settled < requested || (launch !== null && settled < launch))))
    throw new Error('full-access canary timestamps are contradictory');
  const currentAuthority = attempt.authorizationVersion === 'danger-full-access/unrestricted-network/v1' && attempt.authorizationCorrelationId !== null && attempt.correlationId !== null;
  const acceptedOrUnobserved = attempt.launchAcceptedAt === null
    ? attempt.processActivity === 'unobserved'
    : attempt.processActivity === 'launch_accepted' || attempt.processActivity === 'terminal_observed';
  const interruptedProcess = attempt.launchAcceptedAt === null
    ? attempt.processActivity === 'unobserved'
    : attempt.processActivity === 'launch_accepted';
  const cleanupSettled = attempt.cleanupDisposition === 'removed' || attempt.cleanupDisposition === 'failed';
  if (!['legacy_unverified', 'recovered_unobserved'].includes(attempt.disposition) && !currentAuthority)
    throw new Error('full-access canary lacks current durable authorization facts');
  if (!['legacy_unverified', 'recovered_unobserved'].includes(attempt.disposition) && attempt.deadlineAt === null)
    throw new Error('current full-access canary lacks a deadline');
  if (attempt.disposition === 'pending' && (attempt.settledAt !== null || attempt.cleanupDisposition !== 'pending' || attempt.terminalClassification !== 'not_observed' || attempt.terminalExitCode !== null || attempt.receiptObserved || !acceptedOrUnobserved))
    throw new Error('pending full-access canary is contradictory');
  if (attempt.disposition === 'passed' && (!currentAuthority || attempt.launchAcceptedAt === null || attempt.settledAt === null || attempt.processActivity !== 'terminal_observed' || !attempt.receiptObserved || attempt.cleanupDisposition !== 'removed' || !((attempt.terminalClassification === 'exit_code' && attempt.terminalExitCode !== null) || (attempt.terminalClassification === 'not_observed' && attempt.terminalExitCode === null))))
    throw new Error('passed full-access canary violates its receipt and cleanup invariant');
  if (attempt.disposition === 'launch_failed' && (attempt.launchAcceptedAt !== null || attempt.settledAt === null || attempt.processActivity !== 'unobserved' || attempt.terminalClassification !== 'launch_failed' || attempt.terminalExitCode !== null || attempt.receiptObserved || !cleanupSettled))
    throw new Error('launch-failed full-access canary is contradictory');
  if (attempt.disposition === 'terminal_failed' && (attempt.launchAcceptedAt === null || attempt.settledAt === null || attempt.processActivity !== 'terminal_observed' || !cleanupSettled || !((attempt.terminalClassification === 'receipt_missing' && !attempt.receiptObserved) || (attempt.terminalClassification === 'exit_code' && attempt.terminalExitCode !== null) || (attempt.terminalClassification === 'not_observed' && attempt.terminalExitCode === null))))
    throw new Error('terminal-failed full-access canary is contradictory');
  if (['timed_out', 'cancelled'].includes(attempt.disposition) && (attempt.settledAt === null || !cleanupSettled || attempt.terminalClassification !== attempt.disposition || attempt.terminalExitCode !== null || attempt.receiptObserved || !interruptedProcess))
    throw new Error('interrupted full-access canary is contradictory');
  const legacyRecovered = attempt.authorizationVersion === null && attempt.authorizationCorrelationId === null && attempt.correlationId === null && attempt.launchAcceptedAt === null && attempt.deadlineAt === null && attempt.processActivity === 'unobserved' && attempt.terminalClassification === 'recovered_unobserved' && attempt.terminalExitCode === null && !attempt.receiptObserved && attempt.cleanupDisposition === 'not_observed';
  if (attempt.disposition === 'recovered_unobserved' && !legacyRecovered && (attempt.settledAt === null || !cleanupSettled || attempt.terminalClassification !== 'recovered_unobserved' || attempt.terminalExitCode !== null || attempt.receiptObserved || !interruptedProcess))
    throw new Error('recovered full-access canary is contradictory');
  if (attempt.disposition === 'cleanup_failed' && (attempt.launchAcceptedAt === null || attempt.settledAt === null || attempt.processActivity !== 'terminal_observed' || attempt.terminalClassification !== 'cleanup_failed' || !attempt.receiptObserved || attempt.cleanupDisposition !== 'failed'))
    throw new Error('cleanup-failed full-access canary is contradictory');
  if (attempt.disposition === 'legacy_unverified' && (attempt.authorizationVersion !== null || attempt.authorizationCorrelationId !== null || attempt.correlationId !== null || attempt.launchAcceptedAt !== null || attempt.deadlineAt !== null || attempt.settledAt === null || attempt.processActivity !== 'unobserved' || attempt.providerActivity !== 'unobserved' || attempt.terminalClassification !== 'legacy_unverified' || attempt.terminalExitCode !== null || attempt.receiptObserved || attempt.cleanupDisposition !== 'not_observed'))
    throw new Error('legacy full-access canary is contradictory');
  if (attempt.providerActivity !== 'unobserved') throw new Error('full-access canary provider activity must remain unobserved');
}

function validateSandboxAdoption(adoption: NativeProfileSandboxAdoption) {
  if (adoption.disposition === 'not_verified'
    && adoption.executable === null
    && adoption.version === null
    && adoption.workspaceSandboxSupported === null
    && adoption.windowsSandboxSetupSupported === null
    && adoption.correlationId === null
    && adoption.observedAt === null
    && adoption.elevatedModeObserved === null) return;
  if (adoption.executable === null || adoption.version === null || adoption.correlationId === null
    || adoption.observedAt === null || adoption.workspaceSandboxSupported === null
    || adoption.windowsSandboxSetupSupported === null || adoption.elevatedModeObserved === null)
    throw new Error('sandbox adoption evidence is incomplete');
  requiredRfc3339(adoption.observedAt, 'sandbox adoption observation timestamp');
  if (adoption.disposition === 'verified'
    && (!adoption.workspaceSandboxSupported || !adoption.windowsSandboxSetupSupported || !adoption.elevatedModeObserved))
    throw new Error('verified sandbox adoption violates its invariant');
}
function validateSandboxAdoptionConfirmation(confirmation: NativeProfileSandboxAdoptionConfirmation) {
  if (confirmation.disposition === 'not_confirmed'
    && confirmation.correlationId === null
    && confirmation.confirmedAt === null) return;
  if (confirmation.correlationId === null || confirmation.confirmedAt === null)
    throw new Error('sandbox adoption confirmation evidence is incomplete');
  requiredRfc3339(confirmation.confirmedAt, 'sandbox adoption confirmation timestamp');
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
  keys(profile, ['id', 'homePath', 'ownership', 'lifecycle', 'selected', 'execution', 'loginAttempt', 'setupAttempt', 'sandboxAdoption', 'sandboxAdoptionConfirmation', 'fullAccessCanaryAttempt', 'readiness'], `native profile ${index}`);
  if (typeof profile.selected !== 'boolean') throw new Error(`native profile ${index} selected must be boolean`);
  const readiness = object(profile.readiness, `native profile ${index} readiness`);
  keys(readiness, ['authentication', 'sandboxInitialization', 'workspaceWriteCanary', 'dangerFullAccessCanary', 'mcpReporting', 'attentions'], 'readiness');
  const execution = object(profile.execution, `native profile ${index} execution`);
  const loginAttempt = object(profile.loginAttempt, `native profile ${index} login attempt`);
  const setupAttempt = object(profile.setupAttempt, `native profile ${index} setup attempt`);
  const sandboxAdoption = object(profile.sandboxAdoption, `native profile ${index} sandbox adoption`);
  const sandboxAdoptionConfirmation = object(profile.sandboxAdoptionConfirmation, `native profile ${index} sandbox adoption confirmation`);
  const fullAccessCanaryAttempt = object(profile.fullAccessCanaryAttempt, `native profile ${index} full access canary attempt`);
  keys(execution, ['selectedMode', 'dangerFullAccessAuthorized', 'dangerAuthorization'], `native profile ${index} execution`);
  keys(loginAttempt, ['disposition', 'browserHandoff', 'requestedAt', 'launchAcceptedAt', 'settledAt'], `native profile ${index} login attempt`);
  keys(setupAttempt, ['phase', 'disposition', 'executable', 'version', 'workspaceSandboxSupported', 'correlationId', 'requestedAt', 'launchAcceptedAt', 'deadlineAt', 'settledAt', 'terminalClassification', 'terminalExitCode'], `native profile ${index} setup attempt`);
  keys(sandboxAdoption, ['disposition', 'executable', 'version', 'workspaceSandboxSupported', 'windowsSandboxSetupSupported', 'correlationId', 'observedAt', 'elevatedModeObserved'], `native profile ${index} sandbox adoption`);
  keys(sandboxAdoptionConfirmation, ['disposition', 'correlationId', 'confirmedAt'], `native profile ${index} sandbox adoption confirmation`);
  const dangerAuthorization = object(execution.dangerAuthorization, `native profile ${index} danger authorization`);
  keys(dangerAuthorization, ['disposition', 'authorityScope', 'authorityVersion', 'correlationId', 'authorizedAt', 'revokedAt'], `native profile ${index} danger authorization`);
  keys(fullAccessCanaryAttempt, ['disposition', 'authorizationVersion', 'authorizationCorrelationId', 'correlationId', 'requestedAt', 'launchAcceptedAt', 'deadlineAt', 'settledAt', 'processActivity', 'providerActivity', 'terminalClassification', 'terminalExitCode', 'receiptObserved', 'cleanupDisposition'], `native profile ${index} full access canary attempt`);
  if (typeof execution.dangerFullAccessAuthorized !== 'boolean') throw new Error(`native profile ${index} danger authorization must be boolean`);
  const attentions = object(readiness.attentions, 'attentions');
  keys(attentions, ['authentication', 'sandbox', 'canary', 'mcpReporting', 'continuity', 'cli'], 'attentions');
  const decodedSetupAttempt: NativeProfileSetupAttempt = {
    phase: enumValue(setupAttempt.phase, ['not_requested', 'sandbox_initialization', 'workspace_write_canary'], 'setup attempt phase'),
    disposition: enumValue(setupAttempt.disposition, ['not_requested', 'pending', 'launch_failed', 'terminal_succeeded', 'terminal_failed', 'timed_out', 'cancelled', 'recovered_unobserved', 'legacy_unclassified_failed', 'policy_unsupported'], 'setup attempt disposition'),
    executable: nullableString(setupAttempt.executable, 'setup executable'),
    version: nullableString(setupAttempt.version, 'setup executable version'),
    workspaceSandboxSupported: setupAttempt.workspaceSandboxSupported === null ? null : booleanValue(setupAttempt.workspaceSandboxSupported, 'workspace sandbox capability'),
    correlationId: nullableString(setupAttempt.correlationId, 'setup attempt correlation'),
    requestedAt: nullableString(setupAttempt.requestedAt, 'setup request timestamp'),
    launchAcceptedAt: nullableString(setupAttempt.launchAcceptedAt, 'setup launch timestamp'),
    deadlineAt: nullableString(setupAttempt.deadlineAt, 'setup deadline timestamp'),
    settledAt: nullableString(setupAttempt.settledAt, 'setup settlement timestamp'),
    terminalClassification: enumValue(setupAttempt.terminalClassification, ['not_observed', 'exit_code', 'receipt_missing', 'launch_failed', 'timed_out', 'cancelled', 'recovered_unobserved', 'legacy_unclassified_failed', 'policy_unsupported'], 'setup terminal classification'),
    terminalExitCode: setupAttempt.terminalExitCode === null ? null : integerValue(setupAttempt.terminalExitCode, 'setup terminal exit code'),
  };
  validatePolicyUnsupportedSetupAttempt(decodedSetupAttempt);
  validateReceiptMissingSetupAttempt(decodedSetupAttempt);
  const decodedSandboxAdoption: NativeProfileSandboxAdoption = {
    disposition: enumValue(sandboxAdoption.disposition, ['not_verified', 'verified', 'invalidated'], 'sandbox adoption disposition'),
    executable: nullableString(sandboxAdoption.executable, 'sandbox adoption executable'),
    version: nullableString(sandboxAdoption.version, 'sandbox adoption version'),
    workspaceSandboxSupported: sandboxAdoption.workspaceSandboxSupported === null ? null : booleanValue(sandboxAdoption.workspaceSandboxSupported, 'sandbox adoption workspace capability'),
    windowsSandboxSetupSupported: sandboxAdoption.windowsSandboxSetupSupported === null ? null : booleanValue(sandboxAdoption.windowsSandboxSetupSupported, 'sandbox adoption setup capability'),
    correlationId: nullableString(sandboxAdoption.correlationId, 'sandbox adoption correlation'),
    observedAt: nullableString(sandboxAdoption.observedAt, 'sandbox adoption observation timestamp'),
    elevatedModeObserved: sandboxAdoption.elevatedModeObserved === null ? null : booleanValue(sandboxAdoption.elevatedModeObserved, 'sandbox adoption elevated mode observation'),
  };
  validateSandboxAdoption(decodedSandboxAdoption);
  const decodedSandboxAdoptionConfirmation: NativeProfileSandboxAdoptionConfirmation = {
    disposition: enumValue(sandboxAdoptionConfirmation.disposition, ['not_confirmed', 'confirmed', 'invalidated'], 'sandbox adoption confirmation disposition'),
    correlationId: nullableString(sandboxAdoptionConfirmation.correlationId, 'sandbox adoption confirmation correlation'),
    confirmedAt: nullableString(sandboxAdoptionConfirmation.confirmedAt, 'sandbox adoption confirmation timestamp'),
  };
  validateSandboxAdoptionConfirmation(decodedSandboxAdoptionConfirmation);
  const decodedDangerAuthorization: NativeProfileDangerAuthorization = {
    disposition: enumValue(dangerAuthorization.disposition, ['not_authorized', 'legacy_insufficient', 'authorized', 'revoked', 'foreign'], 'danger authorization disposition'),
    authorityScope: dangerAuthorization.authorityScope === null ? null : enumValue<NativeDangerAuthorizationScope>(dangerAuthorization.authorityScope, ['filesystem_only', 'full_machine_filesystem_and_unrestricted_network'], 'danger authorization scope'),
    authorityVersion: nullableString(dangerAuthorization.authorityVersion, 'danger authorization version'),
    correlationId: nullableString(dangerAuthorization.correlationId, 'danger authorization correlation'),
    authorizedAt: nullableString(dangerAuthorization.authorizedAt, 'danger authorization timestamp'),
    revokedAt: nullableString(dangerAuthorization.revokedAt, 'danger authorization revocation timestamp'),
  };
  validateDangerAuthorization(decodedDangerAuthorization, booleanValue(execution.dangerFullAccessAuthorized, `native profile ${index} danger authorization`));
  const decodedFullAccessCanaryAttempt: NativeProfileFullAccessCanaryAttempt = {
    disposition: enumValue(fullAccessCanaryAttempt.disposition, ['not_requested', 'pending', 'passed', 'launch_failed', 'terminal_failed', 'timed_out', 'cancelled', 'recovered_unobserved', 'cleanup_failed', 'legacy_unverified'], 'full access canary disposition'),
    authorizationVersion: nullableString(fullAccessCanaryAttempt.authorizationVersion, 'full access canary authorization version'),
    authorizationCorrelationId: nullableString(fullAccessCanaryAttempt.authorizationCorrelationId, 'full access canary authorization correlation'),
    correlationId: nullableString(fullAccessCanaryAttempt.correlationId, 'full access canary correlation'),
    requestedAt: nullableString(fullAccessCanaryAttempt.requestedAt, 'full access canary request timestamp'),
    launchAcceptedAt: nullableString(fullAccessCanaryAttempt.launchAcceptedAt, 'full access canary launch timestamp'),
    deadlineAt: nullableString(fullAccessCanaryAttempt.deadlineAt, 'full access canary deadline timestamp'),
    settledAt: nullableString(fullAccessCanaryAttempt.settledAt, 'full access canary settlement timestamp'),
    processActivity: enumValue(fullAccessCanaryAttempt.processActivity, ['unobserved', 'launch_accepted', 'terminal_observed'], 'full access canary process activity'),
    providerActivity: enumValue(fullAccessCanaryAttempt.providerActivity, ['unobserved'], 'full access canary provider activity'),
    terminalClassification: enumValue(fullAccessCanaryAttempt.terminalClassification, ['not_observed', 'exit_code', 'receipt_missing', 'launch_failed', 'timed_out', 'cancelled', 'recovered_unobserved', 'cleanup_failed', 'legacy_unverified'], 'full access canary terminal classification'),
    terminalExitCode: fullAccessCanaryAttempt.terminalExitCode === null ? null : integerValue(fullAccessCanaryAttempt.terminalExitCode, 'full access canary terminal exit code'),
    receiptObserved: booleanValue(fullAccessCanaryAttempt.receiptObserved, 'full access canary receipt observation'),
    cleanupDisposition: enumValue(fullAccessCanaryAttempt.cleanupDisposition, ['pending', 'removed', 'failed', 'not_observed'], 'full access canary cleanup disposition'),
  };
  validateFullAccessCanaryAttempt(decodedFullAccessCanaryAttempt);
  return {
    id: stringValue(profile.id, 'profile id'),
    homePath: absolutePath(profile.homePath, 'profile home path'),
    ownership: enumValue(profile.ownership, ['registered_existing', 'application_dedicated'], 'profile ownership'),
    lifecycle: enumValue(profile.lifecycle, ['active', 'missing_or_moved', 'replaced', 'foreign', 'malformed'], 'profile lifecycle'),
    selected: profile.selected,
    execution: {
      selectedMode: enumValue(execution.selectedMode, ['workspace_write', 'danger_full_access'], 'execution mode'),
      dangerFullAccessAuthorized: execution.dangerFullAccessAuthorized,
      dangerAuthorization: decodedDangerAuthorization,
    },
    loginAttempt: {
      disposition: enumValue(loginAttempt.disposition, ['not_requested', 'pending', 'launch_failed', 'terminal_succeeded', 'terminal_failed', 'cancelled', 'recovered_unobserved'], 'login attempt disposition'),
      browserHandoff: enumValue(loginAttempt.browserHandoff, ['unobserved'], 'browser handoff observation'),
      requestedAt: nullableString(loginAttempt.requestedAt, 'login request timestamp'),
      launchAcceptedAt: nullableString(loginAttempt.launchAcceptedAt, 'login launch timestamp'),
      settledAt: nullableString(loginAttempt.settledAt, 'login settlement timestamp'),
    },
    setupAttempt: decodedSetupAttempt,
    sandboxAdoption: decodedSandboxAdoption,
    sandboxAdoptionConfirmation: decodedSandboxAdoptionConfirmation,
    fullAccessCanaryAttempt: decodedFullAccessCanaryAttempt,
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
  verifyPreprovisionedSandbox(profileId: string): Promise<NativeProfileQuery>;
  confirmPreprovisionedSandboxAdoption(profileId: string): Promise<NativeProfileQuery>;
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
    verifyPreprovisionedSandbox: (profileId) => action('verify_native_profile_preprovisioned_sandbox', id(profileId)),
    confirmPreprovisionedSandboxAdoption: (profileId) => action('confirm_native_profile_preprovisioned_sandbox_adoption', id(profileId)),
    runCanary: (profileId) => action('run_native_profile_workspace_write_canary', id(profileId)),
    runDangerFullAccessCanary: (profileId) => action('run_native_profile_danger_full_access_canary', id(profileId)),
    probeMcp: (profileId) => action('probe_native_profile_mcp_reporting', id(profileId)),
  };
}

export const tauriNativeProfileClient = createNativeProfileClient();
