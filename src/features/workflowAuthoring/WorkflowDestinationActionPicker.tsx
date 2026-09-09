import { useState } from 'react';
import type { OtpCapabilityRefDto, OtpPackageDto } from '../../application/otp';
import { OtpElementPicker } from '../../components/otp/OtpElementPicker';
import { OtpElementDetails } from '../otp/otpElements';
import { capabilityKey, offeredActions } from './otpPresentation';
import { OtpConfigurationEditor } from './OtpConfigurationEditor';

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
      <div className="otp-selection">
        <span className="otp-selection__summary">
          {selected?.tool.name ?? 'Unavailable action'}
        </span>
        <button type="button" onClick={() => setOpen(true)}>
          Set action
        </button>
      </div>
      {selected && (
        <OtpConfigurationEditor
          fields={selected.tool.configuration}
          value={configuration}
          onChange={(value) => onChange(action, value)}
        />
      )}
      {open && (
        <OtpElementPicker
          title="Set action"
          selected={[capabilityKey(action)]}
          groups={packages
            .map((pkg) => ({
              id: pkg.id,
              label: pkg.id,
              items: actions
                .filter((a) => a.ref.package === pkg.id)
                .map((a) => ({ id: capabilityKey(a.ref), label: a.tool.name })),
            }))
            .filter((g) => g.items.length)}
          renderDetails={(id) => (
            <OtpElementDetails tool={actions.find((a) => capabilityKey(a.ref) === id)!.tool} />
          )}
          onClose={() => setOpen(false)}
          onConfirm={([id]) => {
            const item = actions.find((a) => capabilityKey(a.ref) === id)!;
            onChange(item.ref, id === capabilityKey(action) ? configuration : {});
            setOpen(false);
          }}
        />
      )}
    </>
  );
}
