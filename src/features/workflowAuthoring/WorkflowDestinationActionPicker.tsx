import { useState } from 'react';
import type { OtpCapabilityRefDto, OtpPackageDto } from '../../application/otp';
import { GroupedElementPicker } from '../../components/otp/GroupedElementPicker';
import { OtpElementDetails } from '../otp/otpElements';
import { capabilityKey, offeredActions } from './otpPresentation';
import { OtpConfigurationEditor } from './OtpConfigurationEditor';
import { WorkflowPromptAgentConfigurationEditor } from './WorkflowPromptAgentConfigurationEditor';

export function WorkflowDestinationActionPicker({
  action,
  configuration,
  packages,
  onChange,
}: {
  readonly action: OtpCapabilityRefDto;
  readonly configuration: Readonly<Record<string, unknown>>;
  readonly packages: readonly OtpPackageDto[];
  readonly onChange: (
    action: OtpCapabilityRefDto,
    configuration: Readonly<Record<string, unknown>>,
  ) => void;
}) {
  const [open, setOpen] = useState(false);
  const actions = offeredActions(packages);
  const selected = actions.find((item) => capabilityKey(item.ref) === capabilityKey(action));
  return (
    <>
      <div className="workflow-destination-action__selector">
        <strong>Action type</strong>
        <div className="otp-selection">
          <span className="otp-selection__summary">
            {selected?.tool.name ?? 'Unavailable action'}
          </span>
          <button type="button" onClick={() => setOpen(true)}>
            Set action
          </button>
        </div>
      </div>
      {selected ? (
        <section className="workflow-destination-action__configuration">
          {action.package === 'workflow' && action.tool === 'prompt_agent' ? (
            <WorkflowPromptAgentConfigurationEditor
              value={configuration}
              onChange={(value) => onChange(action, value)}
            />
          ) : (
            <OtpConfigurationEditor
              fields={selected.tool.configuration}
              value={configuration}
              onChange={(value) => onChange(action, value)}
            />
          )}
        </section>
      ) : null}
      {open && (
        <GroupedElementPicker
          title="Set action"
          selected={[capabilityKey(action)]}
          selectionMode="single"
          groups={packages.map((pkg) => ({
            id: pkg.id,
            label: pkg.name,
            emptyLabel: 'No actions',
            items: actions
              .filter((a) => a.ref.package === pkg.id)
              .map((a) => ({ value: capabilityKey(a.ref), label: a.tool.name })),
          }))}
          renderDetails={(id) => (
            <OtpElementDetails tool={actions.find((a) => capabilityKey(a.ref) === id)!.tool} />
          )}
          onClose={() => setOpen(false)}
          onConfirm={([id]) => {
            const item = actions.find((a) => capabilityKey(a.ref) === id);
            if (!item) return;
            onChange(item.ref, id === capabilityKey(action) ? configuration : {});
            setOpen(false);
          }}
        />
      )}
    </>
  );
}
