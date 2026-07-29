import { useId, useState } from 'react';
import type {
  AutomaticContinuationPolicyUpdateIntent,
  SprintAutomaticContinuationPolicyController,
} from '../../../application/orchestrations';
import '../styles/sprintWorkspace.css';

export interface SprintContinuationControlProps {
  readonly automaticEnabled: boolean;
  readonly policyUpdateIntent?: Extract<
    AutomaticContinuationPolicyUpdateIntent,
    { readonly level: 'sprint' }
  >;
  readonly controller?: SprintAutomaticContinuationPolicyController;
}

/** Recorded discovery interaction. It cannot evaluate or start planning work. */
export function SprintContinuationControl({
  automaticEnabled,
  policyUpdateIntent,
  controller,
}: SprintContinuationControlProps) {
  const [outcome, setOutcome] = useState('');
  const descriptionId = useId();
  const updatePolicy = async () => {
    if (!controller || !policyUpdateIntent) {
      setOutcome(
        'Sprint automatic-continuation policy updates are unsupported because no durable policy store is connected.',
      );
      return;
    }
    const result = await controller.updatePolicy({
      ...policyUpdateIntent,
      automaticEnabled: !automaticEnabled,
    });
    setOutcome(result.message);
  };

  return (
    <div className="sprint-auto-flow">
      <label className="sprint-auto-flow__switch">
        <span>
          <strong>Sprint Auto-flow</strong>
        </span>
        <input
          type="checkbox"
          role="switch"
          checked={automaticEnabled}
          aria-describedby={descriptionId}
          onChange={() => void updatePolicy()}
        />
        <i aria-hidden="true" />
      </label>
      <span className="visually-hidden" id={descriptionId}>
        Controls whether accepted child Work Units should start the next planning round.
      </span>
      <p role="status" aria-live="polite">
        {outcome}
      </p>
    </div>
  );
}
