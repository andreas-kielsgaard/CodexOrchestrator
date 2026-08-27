import { invoke } from '@tauri-apps/api/core';
import type {
  CreateHarnessInput,
  HarnessCatalogItem,
  HarnessConfiguration,
  HarnessDetails,
  HarnessDraft,
  HarnessManagementClient,
  HarnessVersionRef,
  HarnessVersionReplacement,
  HarnessVersionScope,
  LoadHarnessInput,
  OrderHarnessVersionReplacementInput,
  PublishedHarnessVersion,
  PublishHarnessDraftInput,
  PublishSessionHarnessOverrideInput,
  RenameHarnessInput,
  ResolvedHarnessVersion,
  ResolveHarnessVersionInput,
  SaveHarnessDraftInput,
} from '../../application/harnesses';
import {
  createHarnessId,
  createHarnessVersionNumber,
  createHarnessVersionRef,
  harnessVersionRefsEqual,
} from '../../application/harnesses';

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createTauriHarnessManagementClient(
  invokeCommand: Invoke = invoke,
): HarnessManagementClient {
  return {
    async list() {
      return decodeHarnessCatalog(await invokeCommand<unknown>('list_harnesses'));
    },
    async load(input: LoadHarnessInput) {
      const harnessId = decodeHarnessId(input.harnessId, 'Harness ID');
      return decodeHarnessDetails(
        await invokeCommand<unknown>('load_harness', { input: { harnessId } }),
      );
    },
    async create(input: CreateHarnessInput) {
      const name = trimmedString(input.name, 'Harness name');
      const initialConfiguration = decodeHarnessConfiguration(input.initialConfiguration);
      return decodeHarnessCatalogItem(
        await invokeCommand<unknown>('create_harness', {
          input: { name, initialConfiguration },
        }),
      );
    },
    async rename(input: RenameHarnessInput) {
      const harnessId = decodeHarnessId(input.harnessId, 'Harness ID');
      const name = trimmedString(input.name, 'Harness name');
      return decodeHarnessCatalogItem(
        await invokeCommand<unknown>('rename_harness', { input: { harnessId, name } }),
      );
    },
    async saveDraft(input: SaveHarnessDraftInput) {
      const harnessId = decodeHarnessId(input.harnessId, 'Harness ID');
      const basedOn =
        input.basedOn === null ? null : decodeHarnessVersionRef(input.basedOn, 'draft base');
      if (basedOn !== null && basedOn.harnessId !== harnessId)
        throw new Error('Harness draft base belongs to a different Harness');
      const configuration = decodeHarnessConfiguration(input.configuration);
      return decodeHarnessDraft(
        await invokeCommand<unknown>('save_harness_draft', {
          input: { harnessId, basedOn, configuration },
        }),
      );
    },
    async publishDraft(input: PublishHarnessDraftInput) {
      const harnessId = decodeHarnessId(input.harnessId, 'Harness ID');
      return decodePublishedHarnessVersion(
        await invokeCommand<unknown>('publish_harness_draft', { input: { harnessId } }),
      );
    },
    async publishSessionOverride(input: PublishSessionHarnessOverrideInput) {
      const harnessId = decodeHarnessId(input.harnessId, 'Harness ID');
      const sessionId = trimmedString(input.sessionId, 'Agent Session ID');
      const configuration = decodeHarnessConfiguration(input.configuration);
      const version = decodePublishedHarnessVersion(
        await invokeCommand<unknown>('publish_session_harness_override', {
          input: { harnessId, sessionId, configuration },
        }),
      );
      if (
        version.reference.harnessId !== harnessId ||
        version.scope.kind !== 'session_specific' ||
        version.scope.sessionId !== sessionId
      )
        throw new Error('Published Session Harness override does not match its request');
      return version;
    },
    async orderReplacement(input: OrderHarnessVersionReplacementInput) {
      const source = decodeHarnessVersionRef(input.source, 'replacement source');
      const target = decodeHarnessVersionRef(input.target, 'replacement target');
      validateReplacement(source, target);
      const replacement = decodeHarnessVersionReplacement(
        await invokeCommand<unknown>('order_harness_version_replacement', {
          input: { source, target },
        }),
      );
      if (
        !harnessVersionRefsEqual(replacement.source, source) ||
        !harnessVersionRefsEqual(replacement.target, target)
      )
        throw new Error('Ordered Harness replacement does not match its request');
      return replacement;
    },
    async resolveVersion(input: ResolveHarnessVersionInput) {
      const requested = decodeHarnessVersionRef(input.requested, 'requested Harness version');
      const resolution = decodeResolvedHarnessVersion(
        await invokeCommand<unknown>('resolve_harness_version', { input: { requested } }),
      );
      if (!harnessVersionRefsEqual(resolution.requested, requested))
        throw new Error('Harness resolution does not match its request');
      return resolution;
    },
  };
}

export const tauriHarnessManagementClient = createTauriHarnessManagementClient();

export function decodeHarnessCatalog(value: unknown): readonly HarnessCatalogItem[] {
  const harnesses = array(value, 'Harness catalog').map((item, index) =>
    decodeHarnessCatalogItem(item, `Harness catalog item ${index}`),
  );
  requireUnique(
    harnesses.map(({ harnessId }) => harnessId),
    'Harness catalog IDs',
  );
  return harnesses;
}

export function decodeHarnessCatalogItem(value: unknown, label = 'Harness'): HarnessCatalogItem {
  const item = exactObject(value, ['id', 'metadata', 'createdAt', 'updatedAt'], label);
  const metadata = exactObject(item.metadata, ['name'], `${label} metadata`);
  return {
    harnessId: decodeHarnessId(item.id, `${label} ID`),
    name: trimmedString(metadata.name, `${label} name`),
    createdAt: timestamp(item.createdAt, `${label} created timestamp`),
    updatedAt: timestamp(item.updatedAt, `${label} updated timestamp`),
  };
}

export function decodeHarnessDetails(value: unknown): HarnessDetails {
  const root = exactObject(value, ['harness', 'draft', 'versions'], 'Harness details');
  const harness = decodeHarnessCatalogItem(root.harness);
  const draft = root.draft === null ? null : decodeHarnessDraft(root.draft);
  const versions = array(root.versions, 'Harness versions').map((item, index) =>
    decodePublishedHarnessVersion(item, `Harness version ${index}`),
  );
  if (draft !== null && draft.harnessId !== harness.harnessId)
    throw new Error('Harness draft belongs to a different Harness');
  if (versions.some(({ reference }) => reference.harnessId !== harness.harnessId))
    throw new Error('Harness details contain a version from a different Harness');
  requireUnique(
    versions.map(({ reference }) => reference.version),
    'Harness version numbers',
  );
  return { harness, draft, versions };
}

export function decodeHarnessDraft(value: unknown): HarnessDraft {
  const root = exactObject(
    value,
    ['harnessId', 'basedOnVersion', 'configuration', 'draftRevision', 'savedAt'],
    'Harness draft',
  );
  const harnessId = decodeHarnessId(root.harnessId, 'Harness draft ID');
  const basedOn =
    root.basedOnVersion === null
      ? null
      : createHarnessVersionRef(
          harnessId,
          decodeVersionNumber(root.basedOnVersion, 'Harness draft base version'),
        );
  positiveInteger(root.draftRevision, 'Harness draft storage revision');
  return {
    harnessId,
    basedOn,
    configuration: decodeHarnessConfiguration(root.configuration),
    savedAt: timestamp(root.savedAt, 'Harness draft saved timestamp'),
  };
}

export function decodePublishedHarnessVersion(
  value: unknown,
  label = 'Published Harness version',
): PublishedHarnessVersion {
  const root = exactObject(
    value,
    ['reference', 'scope', 'configuration', 'configurationDigest', 'createdAt'],
    label,
  );
  const digest = trimmedString(root.configurationDigest, `${label} digest`);
  if (!/^[0-9a-f]{64}$/.test(digest))
    throw new Error(`${label} digest must be a lowercase SHA-256 digest`);
  return {
    reference: decodeHarnessVersionRef(root.reference, `${label} reference`),
    scope: decodeHarnessVersionScope(root.scope, `${label} scope`),
    configuration: decodeHarnessConfiguration(root.configuration),
    createdAt: timestamp(root.createdAt, `${label} created timestamp`),
  };
}

export function decodeHarnessVersionReplacement(value: unknown): HarnessVersionReplacement {
  const root = exactObject(value, ['source', 'target'], 'Harness version replacement');
  const source = decodeHarnessVersionRef(root.source, 'replacement source');
  const target = decodeHarnessVersionRef(root.target, 'replacement target');
  validateReplacement(source, target);
  return { source, target };
}

export function decodeResolvedHarnessVersion(value: unknown): ResolvedHarnessVersion {
  const root = exactObject(
    value,
    ['requested', 'version', 'replacementPath'],
    'Harness resolution',
  );
  const requested = decodeHarnessVersionRef(root.requested, 'requested Harness version');
  const version = decodePublishedHarnessVersion(root.version, 'resolved Harness version');
  const replacementPath = array(root.replacementPath, 'Harness replacement path').map(
    (item, index) => decodeHarnessVersionRef(item, `Harness replacement path item ${index}`),
  );
  if (replacementPath.length === 0)
    throw new Error('Harness replacement path must include the requested version');
  if (!harnessVersionRefsEqual(replacementPath[0]!, requested))
    throw new Error('Harness replacement path must start with the requested version');
  if (!harnessVersionRefsEqual(replacementPath[replacementPath.length - 1]!, version.reference))
    throw new Error('Harness replacement path must end with the resolved version');
  if (replacementPath.some(({ harnessId }) => harnessId !== requested.harnessId))
    throw new Error('Harness replacement path crosses Harness identities');
  requireUnique(
    replacementPath.map(({ version: number }) => number),
    'Harness replacement path versions',
  );
  return { requested, version, replacementPath };
}

export function decodeHarnessConfiguration(value: unknown): HarnessConfiguration {
  const root = exactObject(
    value,
    ['identityAssignment', 'promptPrefix', 'skills', 'tools', 'runtime', 'hooks', 'updatePolicy'],
    'Harness configuration',
  );
  return {
    identityAssignment: decodeIdentityAssignment(root.identityAssignment),
    promptPrefix: decodePromptPrefix(root.promptPrefix),
    skills: decodeSkills(root.skills),
    tools: decodeTools(root.tools),
    runtime: decodeRuntime(root.runtime),
    hooks: array(root.hooks, 'Harness hooks').map(decodeHook),
    updatePolicy: decodeUpdatePolicy(root.updatePolicy),
  };
}

function decodeHarnessVersionRef(value: unknown, label: string): HarnessVersionRef {
  const root = exactObject(value, ['harnessId', 'version'], label);
  return createHarnessVersionRef(
    decodeHarnessId(root.harnessId, `${label} Harness ID`),
    decodeVersionNumber(root.version, `${label} number`),
  );
}

function decodeHarnessVersionScope(value: unknown, label: string): HarnessVersionScope {
  const root = object(value, label);
  if (root.kind === 'reusable') {
    exactKeys(root, ['kind'], label);
    return { kind: 'reusable' };
  }
  if (root.kind === 'session_specific') {
    exactKeys(root, ['kind', 'sessionId'], label);
    return { kind: 'session_specific', sessionId: trimmedString(root.sessionId, 'Session ID') };
  }
  throw new Error(`${label} kind is invalid`);
}

function decodeIdentityAssignment(value: unknown): HarnessConfiguration['identityAssignment'] {
  const root = object(value, 'Harness identity assignment');
  if (root.kind === 'unrestricted') {
    exactKeys(root, ['kind'], 'Harness identity assignment');
    return { kind: 'unrestricted' };
  }
  if (root.kind === 'allow_list') {
    exactKeys(root, ['kind', 'identityIds'], 'Harness identity assignment');
    const identityIds = array(root.identityIds, 'Harness identity IDs').map((item, index) =>
      trimmedString(item, `Harness identity ID ${index}`),
    );
    requireUnique(identityIds, 'Harness identity IDs');
    return { kind: 'allow_list', identityIds };
  }
  throw new Error('Harness identity assignment kind is invalid');
}

function decodePromptPrefix(value: unknown): HarnessConfiguration['promptPrefix'] {
  const root = exactObject(
    value,
    ['content', 'initialDelivery', 'contextCompressionDelivery'],
    'Harness prompt prefix',
  );
  return {
    content: nonEmptyString(root.content, 'Harness prompt prefix content'),
    initialDelivery: enumValue(root.initialDelivery, ['prepend'], 'Harness initial delivery'),
    contextCompressionDelivery: enumValue(
      root.contextCompressionDelivery,
      ['deferred'],
      'Harness compression delivery',
    ),
  };
}

function decodeSkills(value: unknown): HarnessConfiguration['skills'] {
  const root = exactObject(value, ['availableDiscoveryPolicy', 'items'], 'Harness skills');
  const items = array(root.items, 'Harness skill items').map((value, index) => {
    const item = exactObject(
      value,
      ['name', 'path', 'purpose', 'useWhen', 'policy'],
      `Harness skill ${index}`,
    );
    return {
      name: trimmedString(item.name, `Harness skill ${index} name`),
      path: trimmedString(item.path, `Harness skill ${index} path`),
      purpose: nonEmptyString(item.purpose, `Harness skill ${index} purpose`),
      useWhen: nonEmptyString(item.useWhen, `Harness skill ${index} use when`),
      policy: enumValue(
        item.policy,
        ['always_applicable', 'initial_ingestion', 'available'],
        `Harness skill ${index} policy`,
      ),
    };
  });
  requireUnique(
    items.map(({ name }) => name),
    'Harness skill names',
  );
  return {
    availableDiscoveryPolicy: decodeDiscoveryPolicy(root.availableDiscoveryPolicy),
    items,
  };
}

function decodeTools(value: unknown): HarnessConfiguration['tools'] {
  const root = exactObject(
    value,
    ['availableDiscoveryPolicy', 'items', 'schemaBoundary', 'mcpServers'],
    'Harness tools',
  );
  const items = array(root.items, 'Harness tool items').map((value, index) => {
    const item = exactObject(value, ['name', 'policy'], `Harness tool ${index}`);
    return {
      name: trimmedString(item.name, `Harness tool ${index} name`),
      policy: enumValue(
        item.policy,
        ['every_invocation', 'initial_invocation', 'available'],
        `Harness tool ${index} policy`,
      ),
    };
  });
  requireUnique(
    items.map(({ name }) => name),
    'Harness tool names',
  );
  const mcpServers = array(root.mcpServers, 'Harness MCP servers').map((value, index) => {
    const server = exactObject(value, ['serverName', 'access'], `Harness MCP server ${index}`);
    const access = object(server.access, `Harness MCP server ${index} access`);
    const decodedAccess =
      access.kind === 'entire_server'
        ? (() => {
            exactKeys(access, ['kind'], `Harness MCP server ${index} access`);
            return { kind: 'entire_server' as const };
          })()
        : access.kind === 'selected_tools'
          ? (() => {
              exactKeys(access, ['kind', 'toolNames'], `Harness MCP server ${index} access`);
              const toolNames = array(
                access.toolNames,
                `Harness MCP server ${index} selected tools`,
              ).map((item, toolIndex) =>
                trimmedString(item, `Harness MCP server ${index} tool ${toolIndex}`),
              );
              requireUnique(toolNames, `Harness MCP server ${index} selected tools`);
              return { kind: 'selected_tools' as const, toolNames };
            })()
          : null;
    if (decodedAccess === null)
      throw new Error(`Harness MCP server ${index} access kind is invalid`);
    return {
      serverName: trimmedString(server.serverName, `Harness MCP server ${index} name`),
      access: decodedAccess,
    };
  });
  requireUnique(
    mcpServers.map(({ serverName }) => serverName),
    'Harness MCP server names',
  );
  return {
    availableDiscoveryPolicy: decodeDiscoveryPolicy(root.availableDiscoveryPolicy),
    items,
    schemaBoundary: nonEmptyString(root.schemaBoundary, 'Harness tool schema boundary'),
    mcpServers,
  };
}

function decodeRuntime(value: unknown): HarnessConfiguration['runtime'] {
  const root = exactObject(
    value,
    ['preferredModel', 'sandbox', 'approvalPolicy', 'authoritySummary'],
    'Harness runtime',
  );
  let preferredModel: HarnessConfiguration['runtime']['preferredModel'] = null;
  if (root.preferredModel !== null) {
    const preference = exactObject(
      root.preferredModel,
      ['modelId', 'reasoning'],
      'Harness preferred model',
    );
    preferredModel = {
      modelId: trimmedString(preference.modelId, 'Harness preferred model ID'),
      reasoning:
        preference.reasoning === null
          ? null
          : enumValue(
              preference.reasoning,
              ['low', 'medium', 'high', 'xhigh'],
              'Harness preferred model reasoning',
            ),
    };
  }
  return {
    preferredModel,
    sandbox: enumValue(
      root.sandbox,
      ['read_only', 'workspace_write', 'danger_full_access'],
      'Harness sandbox',
    ),
    approvalPolicy: enumValue(root.approvalPolicy, ['never'], 'Harness approval policy'),
    authoritySummary: nonEmptyString(root.authoritySummary, 'Harness authority summary'),
  };
}

function decodeHook(value: unknown, index: number): HarnessConfiguration['hooks'][number] {
  const root = exactObject(value, ['name', 'status', 'detail'], `Harness hook ${index}`);
  return {
    name: trimmedString(root.name, `Harness hook ${index} name`),
    status: enumValue(
      root.status,
      ['exposed', 'proposed', 'not_connected'],
      `Harness hook ${index} status`,
    ),
    detail: nonEmptyString(root.detail, `Harness hook ${index} detail`),
  };
}

function decodeUpdatePolicy(value: unknown): HarnessConfiguration['updatePolicy'] {
  const root = object(value, 'Harness update policy');
  if (root.status === 'not_configured') {
    exactKeys(root, ['status', 'reason'], 'Harness update policy');
    return {
      status: 'not_configured',
      reason: nonEmptyString(root.reason, 'Harness update policy reason'),
    };
  }
  if (root.status === 'configured') {
    exactKeys(
      root,
      [
        'status',
        'delivery',
        'avoidDuplicateGuidance',
        'notifyRemovedItems',
        'promptReconstruction',
      ],
      'Harness update policy',
    );
    return {
      status: 'configured',
      delivery: enumValue(root.delivery, ['next_prompt'], 'Harness update delivery'),
      avoidDuplicateGuidance: booleanValue(
        root.avoidDuplicateGuidance,
        'Harness duplicate-guidance policy',
      ),
      notifyRemovedItems: booleanValue(root.notifyRemovedItems, 'Harness removed-items policy'),
      promptReconstruction: enumValue(
        root.promptReconstruction,
        ['deferred'],
        'Harness prompt reconstruction',
      ),
    };
  }
  throw new Error('Harness update policy status is invalid');
}

function decodeDiscoveryPolicy(value: unknown): 'whitelist' | 'blacklist' {
  return enumValue(value, ['whitelist', 'blacklist'], 'Harness discovery policy');
}

function decodeHarnessId(value: unknown, label: string) {
  return createHarnessId(trimmedString(value, label));
}

function decodeVersionNumber(value: unknown, label: string) {
  return createHarnessVersionNumber(positiveInteger(value, label));
}

function validateReplacement(source: HarnessVersionRef, target: HarnessVersionRef): void {
  if (source.harnessId !== target.harnessId)
    throw new Error('Harness replacement cannot cross Harness identities');
  if (source.version === target.version)
    throw new Error('Harness replacement must target a different version');
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error(`${label} must be an object`);
  return value as Record<string, unknown>;
}

function exactObject(
  value: unknown,
  allowed: readonly string[],
  label: string,
): Record<string, unknown> {
  const result = object(value, label);
  exactKeys(result, allowed, label);
  return result;
}

function exactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  label: string,
): void {
  const present = Object.keys(value);
  for (const key of present)
    if (!allowed.includes(key)) throw new Error(`${label} contains unknown field: ${key}`);
  for (const key of allowed)
    if (!Object.hasOwn(value, key)) throw new Error(`${label} is missing field: ${key}`);
}

function array(value: unknown, label: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array`);
  return value;
}

function trimmedString(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.length === 0 || value.trim() !== value)
    throw new Error(`${label} must be a non-empty trimmed string`);
  return value;
}

function nonEmptyString(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.trim().length === 0)
    throw new Error(`${label} must be a non-empty string`);
  return value;
}

function enumValue<const T extends string>(
  value: unknown,
  allowed: readonly T[],
  label: string,
): T {
  if (typeof value !== 'string' || !allowed.includes(value as T))
    throw new Error(`${label} is invalid`);
  return value as T;
}

function booleanValue(value: unknown, label: string): boolean {
  if (typeof value !== 'boolean') throw new Error(`${label} must be boolean`);
  return value;
}

function positiveInteger(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 1)
    throw new Error(`${label} must be a positive safe integer`);
  return value;
}

function timestamp(value: unknown, label: string): string {
  const result = trimmedString(value, label);
  if (
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(result) ||
    Number.isNaN(Date.parse(result))
  )
    throw new Error(`${label} must be an RFC3339 timestamp`);
  return result;
}

function requireUnique(values: readonly (string | number)[], label: string): void {
  if (new Set(values).size !== values.length) throw new Error(`${label} must be unique`);
}
