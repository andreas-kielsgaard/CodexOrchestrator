import type {
  CapabilitySetDto,
  RuntimeSelectionsDto,
} from '../../application/executionConfiguration';

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function defaultsWithinCapabilities(
  capabilities: CapabilitySetDto,
  locked?: RuntimeSelectionsDto,
): RuntimeSelectionsDto {
  return {
    model:
      locked?.model && capabilities.models.includes(locked.model)
        ? locked.model
        : (capabilities.models[0] ?? null),
    reasoningMode:
      locked?.reasoningMode && capabilities.reasoningModes.includes(locked.reasoningMode)
        ? locked.reasoningMode
        : (capabilities.reasoningModes[0] ?? null),
    sandboxMode:
      locked?.sandboxMode && capabilities.sandboxModes.includes(locked.sandboxMode)
        ? locked.sandboxMode
        : (capabilities.sandboxModes[0] ?? null),
  };
}
