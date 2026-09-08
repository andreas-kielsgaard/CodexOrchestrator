import type {
  OtpCapabilityRefDto,
  OtpOutputRefDto,
  OtpPackageDto,
} from '../../application/workflowAuthoring';

export const capabilityKey = (ref: OtpCapabilityRefDto) => JSON.stringify([ref.package, ref.tool]);
export const outputKey = (ref: OtpOutputRefDto) =>
  JSON.stringify([ref.capability.package, ref.capability.tool, ref.output]);
export function offeredOutputs(packages: readonly OtpPackageDto[]) {
  return packages.flatMap((pkg) =>
    pkg.tools
      .filter((tool) => tool.entrypoint.kind !== 'action')
      .flatMap((tool) =>
        tool.outputs
          .filter((output) => output.kind === 'data')
          .map((output) => ({
            ref: { capability: { package: pkg.id, tool: tool.id }, output: output.id },
            label: tool.name,
            output,
          })),
      ),
  );
}
export function offeredActions(packages: readonly OtpPackageDto[]) {
  return packages.flatMap((pkg) =>
    pkg.tools
      .filter(
        (tool) =>
          tool.entrypoint.kind === 'action' &&
          tool.outputs.some((output) => output.kind === 'session_request'),
      )
      .map((tool) => ({
        ref: { package: pkg.id, tool: tool.id },
        tool,
      })),
  );
}
