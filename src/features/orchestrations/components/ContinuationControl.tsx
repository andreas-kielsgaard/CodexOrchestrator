import { useId, useState } from 'react';
import type { EpicAutomaticContinuationPolicyController } from '../../../application/orchestrations';
import type { ContinuationPresentation } from '../orchestrationModel';
import '../styles/continuationControl.css';

export interface ContinuationControlProps {
  readonly continuation: ContinuationPresentation;
  readonly controller?: EpicAutomaticContinuationPolicyController;
}

/** The canonical read owns control state; interaction only asks the injected controller. */
export function ContinuationControl({ continuation, controller }: ContinuationControlProps) {
  const [outcome, setOutcome] = useState('');
  const [tooltipVisible, setTooltipVisible] = useState(false);
  const tooltipId = useId();
  const updatePolicy = async () => {
    if (!controller || !continuation.policyUpdateIntent) {
      setOutcome(
        'Automatic-continuation policy updates are unsupported because no durable policy store is connected.',
      );
      return;
    }
    const result = await controller.updatePolicy({
      ...continuation.policyUpdateIntent,
      automaticEnabled: !continuation.automaticEnabled,
    });
    setOutcome(result.message);
  };

  return (
    <div
      className="continuation-projection"
      onMouseEnter={() => setTooltipVisible(true)}
      onMouseLeave={() => setTooltipVisible(false)}
      onFocus={() => setTooltipVisible(true)}
      onBlur={() => setTooltipVisible(false)}
    >
      <label className="continuation-switch">
        <span className="continuation-switch__label">
          <span
            className={`continuation-switch__status-cue${continuation.automaticEnabled ? ' continuation-switch__status-cue--enabled' : ''}`}
            aria-hidden="true"
          />
          Auto-flow
        </span>
        <input
          type="checkbox"
          role="switch"
          checked={continuation.automaticEnabled}
          aria-describedby={tooltipId}
          onChange={() => void updatePolicy()}
        />
        <i aria-hidden="true" />
      </label>
      {tooltipVisible && (
        <span className="continuation-tooltip" id={tooltipId} role="tooltip">
          Automatically starts the next Sprint when the current one finishes.
        </span>
      )}
      <p role="status" aria-live="polite">
        {outcome}
      </p>
    </div>
  );
}
