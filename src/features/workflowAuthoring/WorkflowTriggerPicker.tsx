import { useState } from 'react';
import type {
  OtpPackageDto,
  WorkflowAuthoringConnectionDto,
} from '../../application/workflowAuthoring';
import { GroupedElementPicker } from '../../components/otp/GroupedElementPicker';
import { OtpElementDetails } from '../otp/otpElements';
import { offeredOutputs, outputKey } from './otpPresentation';

export function WorkflowTriggerPicker({
  connection,
  packages,
  onChange,
}: {
  readonly connection: WorkflowAuthoringConnectionDto;
  readonly packages: readonly OtpPackageDto[];
  readonly onChange: (connection: WorkflowAuthoringConnectionDto) => void;
}) {
  const [open, setOpen] = useState(false);
  const outputs = offeredOutputs(packages);
  const selected = outputs.find((item) => outputKey(item.ref) === outputKey(connection.trigger));
  return (
    <>
      <div className="otp-selection">
        <span className="otp-selection__summary">{selected?.label ?? 'Unavailable output'}</span>
        <button type="button" onClick={() => setOpen(true)}>
          Set trigger
        </button>
      </div>
      {open && (
        <GroupedElementPicker
          title="Set trigger"
          selected={[outputKey(connection.trigger)]}
          selectionMode="single"
          groups={packages.map((pkg) => ({
            id: pkg.id,
            label: pkg.name,
            emptyLabel: 'No triggers',
            items: outputs
              .filter((o) => o.packageId === pkg.id)
              .map((o) => ({ value: outputKey(o.ref), label: o.label })),
          }))}
          renderDetails={(id) => {
            const item = outputs.find((o) => outputKey(o.ref) === id)!;
            const removed = connection.promptInputs.filter(
              (input) =>
                input.kind === 'output_field' &&
                !Object.hasOwn(item.output.schema.properties ?? {}, input.field),
            );
            return (
              <>
                {removed.length > 0 && (
                  <p role="status">
                    Applying this trigger removes unavailable field inputs:{' '}
                    {removed.map((i) => (i.kind === 'output_field' ? i.field : '')).join(', ')}.
                  </p>
                )}
                <OtpElementDetails tool={item.tool} output={item.output} />
              </>
            );
          }}
          onClose={() => setOpen(false)}
          onConfirm={([id]) => {
            const item = outputs.find((o) => outputKey(o.ref) === id);
            if (!item) return;
            onChange({
              ...connection,
              trigger: item.ref,
              promptInputs: connection.promptInputs.filter(
                (input) =>
                  input.kind !== 'output_field' ||
                  Object.hasOwn(item.output.schema.properties ?? {}, input.field),
              ),
            });
            setOpen(false);
          }}
        />
      )}
    </>
  );
}
