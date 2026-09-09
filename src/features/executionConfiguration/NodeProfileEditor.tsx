import { RuntimeDefaultsFields } from './RuntimeDefaultsFields';
import { useState } from 'react';
import {
  CatalogSingleSelect,
  type CatalogOption,
  type CatalogState,
} from '../../components/CatalogSelect';
import { CollapsibleSection } from '../../components/CollapsibleSection';
import { MarkdownEditor } from '../../components/MarkdownEditor';
import { ValidationSummary } from '../../components/ValidationSummary';
import { AgentIdentityBadge } from '../identities';
import { CapabilitySetFields } from './CapabilitySetFields';
import type {
  AgentIdentityOption,
  CapabilityProfileOption,
  NodeProfileCopySource,
  NodeProfileEditorValue,
  RuntimeCapabilityCatalogs,
  RuntimeSelectionsViewModel,
} from './types';
import './executionConfiguration.css';

export interface NodeProfileEditorProps {
  readonly value: NodeProfileEditorValue;
  readonly capabilityProfiles: readonly CapabilityProfileOption[];
  readonly identities: readonly AgentIdentityOption[];
  readonly capabilityCatalogs: RuntimeCapabilityCatalogs;
  readonly copySources?: readonly NodeProfileCopySource[];
  readonly runtimeLockedSelections?: RuntimeSelectionsViewModel;
  readonly validationErrors?: readonly string[];
  onChange(value: NodeProfileEditorValue): void;
  onCopyFromNode?(sourceNodeId: string): void;
}

/** Workflow-owned node configuration. Copying is requested through a caller-owned action port. */
export function NodeProfileEditor({
  value,
  capabilityProfiles,
  identities,
  capabilityCatalogs,
  copySources = [],
  runtimeLockedSelections,
  validationErrors = [],
  onChange,
  onCopyFromNode,
}: NodeProfileEditorProps) {
  const [copySourceId, setCopySourceId] = useState<string | null>(null);
  const identity = identities.find((candidate) => candidate.id === value.identityId);

  return (
    <div className="execution-configuration" data-testid="node-profile-editor">
      <header className="execution-configuration__header">
        <div>
          <span>Workflow node</span>
          <h1>{value.nodeName || 'Unnamed node'}</h1>
          <p>Configure the prompt, exposed capabilities, and defaults for Sessions created here.</p>
        </div>
        {identity ? (
          <AgentIdentityBadge identity={identity} secondaryLabel="Agent identity" />
        ) : null}
      </header>

      <ValidationSummary errors={validationErrors} />

      <CollapsibleSection
        title="Node details"
        description="Operational naming and presentation are independent from capability policy."
        className="execution-configuration__section"
      >
        <div className="execution-configuration__field-grid">
          <label className="execution-configuration__field">
            <span>Node name</span>
            <input
              aria-label="Node name"
              value={value.nodeName}
              onChange={(event) => onChange({ ...value, nodeName: event.currentTarget.value })}
            />
          </label>
          <CatalogSingleSelect
            label="Agent identity"
            catalog={asCatalog(
              identities.map((candidate) => ({
                value: candidate.id,
                label: candidate.displayName,
              })),
              'Identity catalog',
            )}
            value={value.identityId}
            emptyLabel="No identity"
            onChange={(identityId) => onChange({ ...value, identityId })}
          />
          <CatalogSingleSelect
            label="Capability profile"
            catalog={asCatalog(
              capabilityProfiles.map((profile) => ({
                value: profile.id,
                label: profile.label,
                description: `Revision ${profile.revision}`,
              })),
              'Capability profiles',
            )}
            value={value.capabilityProfileId}
            emptyLabel="Select a profile"
            onChange={(capabilityProfileId) => onChange({ ...value, capabilityProfileId })}
          />
        </div>
      </CollapsibleSection>

      {copySources.length > 0 && onCopyFromNode ? (
        <CollapsibleSection
          title="Copy node configuration"
          description="Copy creates independent node state that can be edited here."
          className="execution-configuration__section"
          defaultExpanded={false}
        >
          <div className="execution-configuration__copy-row">
            <CatalogSingleSelect
              label="Source node"
              catalog={asCatalog(
                copySources.map((source) => ({
                  value: source.nodeId,
                  label: source.nodeName,
                  description: source.summary,
                })),
                'Current workflow',
              )}
              value={copySourceId}
              emptyLabel="Select a node"
              onChange={setCopySourceId}
            />
            <button
              type="button"
              disabled={!copySourceId}
              onClick={() => copySourceId && onCopyFromNode(copySourceId)}
            >
              Copy configuration
            </button>
          </div>
        </CollapsibleSection>
      ) : null}

      <CollapsibleSection
        title="Initial prompt"
        description="Included when this node creates a Session; omitted from later messages."
        className="execution-configuration__section"
      >
        <MarkdownEditor
          label="Initial prompt"
          value={value.initialPrompt}
          editable
          onChange={(initialPrompt) => onChange({ ...value, initialPrompt })}
        />
      </CollapsibleSection>

      <CollapsibleSection
        title="Exposed capabilities"
        description="Restrict the selected Capability Profile for this node."
        className="execution-configuration__section"
      >
        <CapabilitySetFields
          catalogs={capabilityCatalogs}
          value={value.exposedCapabilities}
          scopeLabel="Sessions created by this node"
          onChange={(exposedCapabilities) => onChange({ ...value, exposedCapabilities })}
        />
      </CollapsibleSection>

      <CollapsibleSection
        title="Pinned defaults"
        description="Workflow-triggered messages use these defaults. Runtime locks remain inherited."
        className="execution-configuration__section"
      >
        <RuntimeDefaultsFields
          catalogs={capabilityCatalogs}
          allowed={value.exposedCapabilities}
          value={value.pinnedDefaults}
          locked={runtimeLockedSelections}
          onChange={(pinnedDefaults) => onChange({ ...value, pinnedDefaults })}
        />
      </CollapsibleSection>
    </div>
  );
}

function asCatalog<T extends string>(
  options: readonly CatalogOption<T>[],
  sourceLabel: string,
): CatalogState<T> {
  return { availability: 'available', options, sourceLabel };
}
