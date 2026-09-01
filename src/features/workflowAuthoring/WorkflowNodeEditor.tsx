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
import { defaultsWithinCapabilities } from './workflowAuthoringPresentation';

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
  const update = (next: NodeProfileEditorValue) => {
    const profileChanged = next.capabilityProfileId !== value.capabilityProfileId;
    const selectedProfile = next.capabilityProfileId
      ? profileValues.get(next.capabilityProfileId)
      : undefined;
    const exposedCapabilities =
      profileChanged && selectedProfile
        ? selectedProfile.allowedCapabilities
        : next.exposedCapabilities;
    const pinnedDefaults =
      profileChanged && selectedProfile
        ? defaultsWithinCapabilities(selectedProfile.allowedCapabilities, runtime.lockedSelections)
        : next.pinnedDefaults;
    onChange({
      ...node,
      name: next.nodeName,
      agentIdentityId: next.identityId,
      capabilityProfileId: next.capabilityProfileId ?? '',
      initialPrompt: next.initialPrompt.trim() ? next.initialPrompt : null,
      nodeProfile: {
        ...node.nodeProfile,
        allowedCapabilities: exposedCapabilities,
        pinnedDefaults,
      },
    });
  };
  return (
    <>
      <NodeProfileEditor
        value={value}
        capabilityProfiles={profiles}
        identities={identities}
        capabilityCatalogs={runtime.catalogs}
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
