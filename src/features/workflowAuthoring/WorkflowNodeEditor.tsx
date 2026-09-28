import {
  capabilityProfileDeviceModels,
  type CapabilityProfileDto,
  type ExecutionConfigurationClient,
  type ProfileModelCatalogueDto,
} from '../../application/executionConfiguration';
import type {
  WorkflowAuthoringNodeDto,
  WorkflowRecipeDraftDto,
} from '../../application/workflowAuthoring';
import {
  NodeProfileEditor,
  nodeProfileCatalogs,
  nodeProfileValidationErrors,
  type AgentIdentityOption,
  type CapabilityProfileOption,
  type NodeProfileEditorValue,
  type RuntimeProfileViewModel,
  type RuntimeCapabilityCatalogs,
} from '../executionConfiguration';
import { AgentMcpConfigurationEditor } from './AgentMcpConfigurationEditor';

function profileModels(
  profile: CapabilityProfileDto | undefined,
  catalogues: Readonly<Record<string, ProfileModelCatalogueDto>>,
) {
  const execution = profile
    ? (profile.execution ??
      profile.routePolicies?.find((route) => route.routeId === profile.defaultRouteId)?.execution ??
      profile.routePolicies?.[0]?.execution)
    : undefined;
  return capabilityProfileDeviceModels(profile, execution, catalogues);
}

function routeAwareCeiling(
  profile: CapabilityProfileDto | undefined,
  catalogues: Readonly<Record<string, ProfileModelCatalogueDto>>,
) {
  if (!profile) return undefined;
  const models = profileModels(profile, catalogues);
  const reasoningModes = [
    ...new Set(models.flatMap((model) => model.reasoningModes.map((mode) => mode.id))),
  ];
  return {
    ...profile.allowedCapabilities,
    models: models.length
      ? [...new Set(models.map((model) => model.id))]
      : profile.allowedCapabilities.models,
    reasoningModes: reasoningModes.length
      ? reasoningModes
      : profile.allowedCapabilities.reasoningModes,
  };
}

export function WorkflowNodeEditor({
  node,
  draft,
  runtime,
  profiles,
  profileValues,
  modelCatalogues,
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
  readonly modelCatalogues: Readonly<Record<string, ProfileModelCatalogueDto>>;
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
  const selectedProfile = profileValues.get(node.capabilityProfileId);
  const routeModels = profileModels(selectedProfile, modelCatalogues);
  const routeReasoningModes = [
    ...new Set(routeModels.flatMap((model) => model.reasoningModes.map((mode) => mode.id))),
  ];
  const ceiling = routeAwareCeiling(selectedProfile, modelCatalogues);
  const routeCatalogs: RuntimeCapabilityCatalogs = routeModels.length
    ? {
        ...runtime.catalogs,
        models: {
          availability: 'available',
          sourceLabel: 'Capability Profile routes',
          options: [...new Map(routeModels.map((model) => [model.id, model])).values()].map(
            (model) => ({
              value: model.id,
              label: model.label,
              description: model.description,
            }),
          ),
        },
        reasoningModes: {
          availability: 'available',
          sourceLabel: 'Capability Profile routes',
          options: routeReasoningModes.map((mode) => ({ value: mode, label: mode })),
        },
      }
    : runtime.catalogs;
  const catalogs = nodeProfileCatalogs(routeCatalogs, ceiling);
  const errors = nodeProfileValidationErrors(node.nodeProfile, ceiling);
  const update = (next: NodeProfileEditorValue) => {
    const profileChanged = next.capabilityProfileId !== value.capabilityProfileId;
    const selectedProfile = next.capabilityProfileId
      ? profileValues.get(next.capabilityProfileId)
      : undefined;
    const exposedCapabilities =
      profileChanged && selectedProfile
        ? (routeAwareCeiling(selectedProfile, modelCatalogues) ??
          selectedProfile.allowedCapabilities)
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
      <AgentMcpConfigurationEditor
        packages={runtime.catalogs.otpPackages ?? []}
        selectedTools={node.nodeProfile.allowedCapabilities.mcpTools}
        value={node.agentMcpConfiguration ?? {}}
        onChange={(agentMcpConfiguration) => onChange({ ...node, agentMcpConfiguration })}
      />
    </>
  );
}
