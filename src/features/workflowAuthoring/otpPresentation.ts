import type {
  OtpCapabilityRefDto,
  OtpOutputRefDto,
  OtpPackageDto,
  WorkflowAuthoringNodeDto,
} from '../../application/workflowAuthoring';

export const capabilityKey = (ref: OtpCapabilityRefDto) => JSON.stringify([ref.package, ref.tool]);
export const outputKey = (ref: OtpOutputRefDto) =>
  JSON.stringify([ref.capability.package, ref.capability.tool, ref.output]);
function nodeDescriptor(packages: readonly OtpPackageDto[], node?: WorkflowAuthoringNodeDto) {
  return node?.kind === 'capability'
    ? packages
        .find((pkg) => pkg.id === node.capability.package)
        ?.nodeCapabilities?.find((item) => item.id === node.capability.tool)
    : undefined;
}
export function offeredOutputs(
  packages: readonly OtpPackageDto[],
  node?: WorkflowAuthoringNodeDto,
) {
  const descriptor = nodeDescriptor(packages, node);
  return packages.flatMap((pkg) =>
    pkg.tools
      .filter(
        (tool) =>
          node?.kind === 'capability' ||
          (tool.entrypoint.kind !== 'action' && tool.entrypoint.kind !== 'service_event'),
      )
      .flatMap((tool) =>
        tool.outputs
          .filter(
            (output) =>
              output.kind === 'data' &&
              (node?.kind !== 'capability' ||
                descriptor?.outputs.some(
                  (ref) =>
                    ref.capability.package === pkg.id &&
                    ref.capability.tool === tool.id &&
                    ref.output === output.id,
                )),
          )
          .map((output) => ({
            ref: { capability: { package: pkg.id, tool: tool.id }, output: output.id },
            label: `${tool.name} · ${output.name}`,
            tool,
            packageId: pkg.id,
            output,
          })),
      ),
  );
}
export function offeredActions(
  packages: readonly OtpPackageDto[],
  node?: WorkflowAuthoringNodeDto,
) {
  const descriptor = nodeDescriptor(packages, node);
  return packages.flatMap((pkg) =>
    pkg.tools
      .filter(
        (tool) =>
          tool.entrypoint.kind === 'action' &&
          (node?.kind === 'capability'
            ? descriptor?.actions.some((ref) => ref.package === pkg.id && ref.tool === tool.id)
            : tool.outputs.some(
                (output) =>
                  output.kind === 'session_request' || output.kind === 'session_stop_request',
              )),
      )
      .map((tool) => ({
        ref: { package: pkg.id, tool: tool.id },
        tool,
      })),
  );
}
