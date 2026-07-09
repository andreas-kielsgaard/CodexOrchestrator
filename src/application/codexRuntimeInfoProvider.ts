import type { CLIInstanceHandler, CLIInstanceSnapshot } from './cliInstanceHandler';
import { CLISessionMaster } from './cliSessionMaster';

export type CodexReasoningEffort = 'minimal' | 'low' | 'medium' | 'high' | 'xhigh';

export interface CodexRuntimeModel {
  id: string;
  name?: string;
  recommended?: boolean;
  defaultReasoningEffort?: CodexReasoningEffort;
  reasoningEfforts?: CodexReasoningEffort[];
}

export interface CodexRuntimeInfo {
  models: CodexRuntimeModel[];
  recommendedModel: string;
  configuredModel?: string;
  modelProvider?: string;
  codexVersion?: string;
  configPath?: string;
  healthStatus?: 'ok' | 'warning' | 'error';
  reasoningEfforts: CodexReasoningEffort[];
  source: 'codex-debug-models' | 'codex-doctor-and-debug-models' | 'fallback';
}

export interface CodexRuntimeInfoProviderOptions {
  createCLIHandler(): CLIInstanceHandler;
}

export const fallbackRuntimeInfo: CodexRuntimeInfo = {
  models: [{ id: 'gpt-5.5', recommended: true }],
  recommendedModel: 'gpt-5.5',
  reasoningEfforts: ['minimal', 'low', 'medium', 'high', 'xhigh'],
  source: 'fallback',
};

export class CodexRuntimeInfoProvider {
  constructor(
    private readonly sessionMaster: CLISessionMaster,
    private readonly options: CodexRuntimeInfoProviderOptions,
  ) {}

  async loadRuntimeInfo(): Promise<CodexRuntimeInfo> {
    const doctorOutput = await this.runCodexCommand(['doctor', '--json']);
    const catalogOutput = await this.runCodexCommand(['debug', 'models', '--bundled']);
    const doctorInfo = doctorOutput ? parseCodexDoctorReport(doctorOutput) : null;
    const catalogInfo = catalogOutput ? parseCodexModelCatalog(catalogOutput) : null;

    if (!catalogInfo && !doctorInfo) {
      return fallbackRuntimeInfo;
    }

    if (!catalogInfo) {
      return {
        ...fallbackRuntimeInfo,
        ...doctorInfo,
        source: doctorInfo ? 'codex-doctor-and-debug-models' : 'fallback',
      };
    }

    return {
      ...catalogInfo,
      ...doctorInfo,
      recommendedModel:
        doctorInfo?.configuredModel ??
        catalogInfo.recommendedModel ??
        fallbackRuntimeInfo.recommendedModel,
      source: doctorInfo ? 'codex-doctor-and-debug-models' : 'codex-debug-models',
    };
  }

  private async runCodexCommand(args: string[]): Promise<string | null> {
    const lease = this.sessionMaster.acquire({
      purpose: 'codex-runtime-info',
      createHandler: this.options.createCLIHandler,
    });

    try {
      const snapshot = await lease.handler.open({
        command: 'codex',
        args,
      });

      if (snapshot.status !== 'completed') {
        return null;
      }

      return snapshotStdout(snapshot);
    } finally {
      await this.sessionMaster.close(lease);
    }
  }
}

export function parseCodexModelCatalog(stdout: string): CodexRuntimeInfo | null {
  if (!stdout.trim()) {
    return null;
  }

  const parsed = parseJson(stdout);
  const modelRecords = findModelRecords(parsed);
  const models = modelRecords.map(normalizeModelRecord).filter(Boolean) as CodexRuntimeModel[];

  if (models.length === 0) {
    return null;
  }

  return {
    models,
    recommendedModel: models.find((model) => model.recommended)?.id ?? models[0].id,
    reasoningEfforts: uniqueReasoningEfforts(models),
    source: 'codex-debug-models',
  };
}

export function parseCodexDoctorReport(stdout: string): Partial<CodexRuntimeInfo> | null {
  if (!stdout.trim()) {
    return null;
  }

  const parsed = parseJson(stdout);
  if (!isRecord(parsed)) {
    return null;
  }

  const checks = parsed.checks;
  const configDetails = doctorDetails(checks, 'config.load');
  const runtimeDetails = doctorDetails(checks, 'runtime.provenance');

  return {
    ...(stringField(configDetails?.model)
      ? { configuredModel: stringField(configDetails?.model) }
      : {}),
    ...(stringField(configDetails?.['model provider'])
      ? { modelProvider: stringField(configDetails?.['model provider']) }
      : {}),
    ...(stringField(configDetails?.['config.toml'])
      ? { configPath: stringField(configDetails?.['config.toml']) }
      : {}),
    ...(stringField(parsed.codexVersion)
      ? { codexVersion: stringField(parsed.codexVersion) }
      : stringField(runtimeDetails?.version)
        ? { codexVersion: stringField(runtimeDetails?.version) }
        : {}),
    ...(doctorHealthStatus(parsed.overallStatus)
      ? { healthStatus: doctorHealthStatus(parsed.overallStatus) }
      : {}),
  };
}

export function extractCodexTechnicalValues(metadata: Record<string, string>): {
  model?: string;
  reasoningEffort?: string;
} {
  return {
    ...(metadata.model ? { model: metadata.model } : {}),
    ...(metadata.reasoningEffort ? { reasoningEffort: metadata.reasoningEffort } : {}),
  };
}

function parseJson(stdout: string): unknown {
  try {
    return JSON.parse(stdout);
  } catch {
    return null;
  }
}

function snapshotStdout(snapshot: CLIInstanceSnapshot): string {
  return snapshot.output
    .filter((chunk) => chunk.stream === 'stdout')
    .map((chunk) => chunk.content)
    .join('\n');
}

function doctorDetails(checks: unknown, id: string): Record<string, unknown> | null {
  if (!isRecord(checks)) {
    return null;
  }

  const check = checks[id];
  if (!isRecord(check) || !isRecord(check.details)) {
    return null;
  }

  return check.details;
}

function doctorHealthStatus(value: unknown): CodexRuntimeInfo['healthStatus'] | undefined {
  return value === 'ok' || value === 'warning' || value === 'error' ? value : undefined;
}

function findModelRecords(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value.filter(isRecord);
  }

  if (!isRecord(value)) {
    return [];
  }

  for (const key of ['models', 'model_catalog', 'modelCatalog', 'items']) {
    const child = value[key];
    if (Array.isArray(child)) {
      return child.filter(isRecord);
    }
  }

  return [];
}

function normalizeModelRecord(record: Record<string, unknown>): CodexRuntimeModel | null {
  const id = stringField(record.id) ?? stringField(record.slug) ?? stringField(record.name);

  if (!id) {
    return null;
  }

  const name =
    stringField(record.display_name) ?? stringField(record.displayName) ?? stringField(record.name);
  const efforts = reasoningEfforts(record);
  const defaultReasoningEffort = reasoningEffortField(
    record.default_reasoning_level ??
      record.defaultReasoningLevel ??
      record.default_reasoning_effort,
  );

  return {
    id,
    ...(name && name !== id ? { name } : {}),
    ...(booleanField(record.recommended) || record.priority === 0 ? { recommended: true } : {}),
    ...(defaultReasoningEffort ? { defaultReasoningEffort } : {}),
    ...(efforts.length > 0 ? { reasoningEfforts: efforts } : {}),
  };
}

function uniqueReasoningEfforts(models: CodexRuntimeModel[]): CodexReasoningEffort[] {
  const efforts = models.flatMap((model) => model.reasoningEfforts ?? []);
  return efforts.length > 0 ? [...new Set(efforts)] : fallbackRuntimeInfo.reasoningEfforts;
}

function reasoningEfforts(record: Record<string, unknown>): CodexReasoningEffort[] {
  const candidates =
    arrayField(record.reasoning_efforts) ??
    arrayField(record.reasoningEfforts) ??
    arrayField(record.supported_reasoning_levels) ??
    arrayField(record.supportedReasoningLevels) ??
    arrayField(record.supported_reasoning_efforts) ??
    [];

  return candidates.map(reasoningEffortFromCatalogValue).filter(isReasoningEffort);
}

function stringField(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value : undefined;
}

function booleanField(value: unknown): boolean {
  return value === true;
}

function arrayField(value: unknown): unknown[] | undefined {
  return Array.isArray(value) ? value : undefined;
}

function reasoningEffortFromCatalogValue(value: unknown): unknown {
  if (isRecord(value)) {
    return value.effort ?? value.level ?? value.id;
  }

  return value;
}

function reasoningEffortField(value: unknown): CodexReasoningEffort | undefined {
  return isReasoningEffort(value) ? value : undefined;
}

function isReasoningEffort(value: unknown): value is CodexReasoningEffort {
  return (
    value === 'minimal' ||
    value === 'low' ||
    value === 'medium' ||
    value === 'high' ||
    value === 'xhigh'
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
