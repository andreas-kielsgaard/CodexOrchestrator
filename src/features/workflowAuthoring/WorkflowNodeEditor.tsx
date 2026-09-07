import type { ExecutionConfigurationClient } from '../../application/executionConfiguration';
import type {
  WorkflowAuthoringNodeDto,
  WorkflowRecipeDraftDto,
} from '../../application/workflowAuthoring';
import {
  NodeProfileEditor,
  RuntimeProfileInspector,
  type AgentIdentityOption,
  type CapabilityProfileOption,
  type NodeProfileEditorValue,
  type RuntimeProfileViewModel,
} from '../executionConfiguration';
import { selectedMcpToolValues } from '../executionConfiguration/types';
import type { CatalogState } from '../../components/CatalogSelect';

export function WorkflowNodeEditor({
  node,
  draft,
  runtime,
  profiles,
  profileValues,
  identities,
  onChange,
  onCopy,
}: {
  readonly node: WorkflowAuthoringNodeDto;
  readonly draft: WorkflowRecipeDraftDto;
  readonly runtime: RuntimeProfileViewModel;
  readonly profiles: readonly CapabilityProfileOption[];
  readonly profileValues: ReadonlyMap<
    string,
    Awaited<ReturnType<ExecutionConfigurationClient['loadCapabilityProfile']>>
  >;
  readonly identities: readonly AgentIdentityOption[];
  readonly onChange: (node: WorkflowAuthoringNodeDto) => void;
  readonly onCopy: (nodeId: string) => void;
}) {
  const value: NodeProfileEditorValue = {
    nodeName: node.name,
    identityId: node.agentIdentityId,
    capabilityProfileId: node.capabilityProfileId || null,
    initialPrompt: node.initialPrompt ?? '',
    exposedCapabilities: node.nodeProfile.allowedCapabilities,
    pinnedDefaults: node.nodeProfile.pinnedDefaults,
  };
  const ceiling = profileValues.get(node.capabilityProfileId)?.allowedCapabilities;
  const restrict = <T extends string>(
    catalog: CatalogState<T>,
    allowed: readonly T[],
  ): CatalogState<T> => ({
    ...catalog,
    options: catalog.options.filter((option) => allowed.includes(option.value)),
  });
  const catalogs = {
    models: restrict(runtime.catalogs.models, ceiling?.models ?? []),
    reasoningModes: restrict(runtime.catalogs.reasoningModes, ceiling?.reasoningModes ?? []),
    sandboxModes: restrict(runtime.catalogs.sandboxModes, ceiling?.sandboxModes ?? []),
    mcpTools: restrict(runtime.catalogs.mcpTools, selectedMcpToolValues(ceiling?.mcpTools ?? {})),
    skills: restrict(runtime.catalogs.skills, ceiling?.skills ?? []),
  };
  const errors: string[] = [];
  if (!ceiling) errors.push('Choose an available Capability Profile.');
  for (const [label, allowed, selected, pinned] of [
    ['model', ceiling?.models ?? [], value.exposedCapabilities.models, value.pinnedDefaults.model],
    [
      'reasoning mode',
      ceiling?.reasoningModes ?? [],
      value.exposedCapabilities.reasoningModes,
      value.pinnedDefaults.reasoningMode,
    ],
    [
      'sandbox mode',
      ceiling?.sandboxModes ?? [],
      value.exposedCapabilities.sandboxModes,
      value.pinnedDefaults.sandboxMode,
    ],
  ] as const) {
    if (selected.some((choice) => !(allowed as readonly string[]).includes(choice)))
      errors.push(`An exposed ${label} is outside the selected profile.`);
    if (pinned && !(selected as readonly string[]).includes(pinned))
      errors.push(
        `Default ${label} “${pinned}” is not exposed. Choose another default or expose it.`,
      );
  }
  const update = (next: NodeProfileEditorValue) => {
    const profileChanged = next.capabilityProfileId !== value.capabilityProfileId;
    const selectedProfile = next.capabilityProfileId
      ? profileValues.get(next.capabilityProfileId)
      : undefined;
    const exposedCapabilities =
      profileChanged && selectedProfile
        ? selectedProfile.allowedCapabilities
        : next.exposedCapabilities;
    onChange({
      ...node,
      name: next.nodeName,
      agentIdentityId: next.identityId,
      capabilityProfileId: next.capabilityProfileId ?? '',
      initialPrompt: next.initialPrompt.trim() ? next.initialPrompt : null,
      nodeProfile: {
        ...node.nodeProfile,
        allowedCapabilities: exposedCapabilities,
        pinnedDefaults: next.pinnedDefaults,
      },
    });
  };
  return (
    <>
      <NodeProfileEditor
        value={value}
        capabilityProfiles={profiles}
        identities={identities}
        capabilityCatalogs={catalogs}
        validationErrors={errors}
        runtimeLockedSelections={runtime.lockedSelections}
        copySources={draft.nodes
          .filter((candidate) => candidate.nodeId !== node.nodeId)
          .map((candidate) => ({ nodeId: candidate.nodeId, nodeName: candidate.name }))}
        onChange={update}
        onCopyFromNode={onCopy}
      />
      <RuntimeProfileInspector runtime={runtime} defaultExpanded={false} />
    </>
  );
}
