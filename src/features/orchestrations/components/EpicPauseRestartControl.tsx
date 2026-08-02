import { useEffect, useState } from 'react';
import type { EpicPauseRestartController, EpicPauseRestartQuery } from '../../../application/orchestrations';

export function EpicPauseRestartControl({ epicId, controller }: { readonly epicId: string; readonly controller?: EpicPauseRestartController }) {
  const [outcome, setOutcome] = useState('');
  const [query, setQuery] = useState<EpicPauseRestartQuery | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const refresh = async () => {
    if (!controller) return;
    try {
      setQuery(await controller.load(epicId));
    } catch {
      setOutcome('Epic controls are unavailable because durable control state could not be loaded.');
    }
  };
  useEffect(() => { void refresh(); }, [controller, epicId]);
  const request = async (kind: 'pause' | 'restart') => {
    if (!controller || submitting) return;
    setSubmitting(true);
    try {
      const result = kind === 'pause' ? await controller.requestPause(epicId) : await controller.requestRestart(epicId);
      setOutcome(`${result.kind === 'pause' ? 'Pause' : 'Restart'} ${result.status}: ${result.launchedCount} of ${result.targetCount} dispatches launch-accepted. Provider receipt, compliance, and progress are not observed.`);
      await refresh();
    } catch {
      setOutcome('The durable Epic control request is unavailable.');
    } finally {
      setSubmitting(false);
    }
  };
  return (
    <div className="epic-pause-restart-control">
      <button type="button" disabled={submitting || !query || query.pause.availability !== 'available'} onClick={() => void request('pause')}>
        Pause
      </button>
      <button type="button" disabled={submitting || !query || query.restart.availability !== 'available'} onClick={() => void request('restart')}>
        Restart
      </button>
      <p role="status" aria-live="polite">
        {outcome || 'Loading durable Epic controls.'}
      </p>
      {query && (
        <dl>
          <dt>Pause</dt>
          <dd>{describe(query.pause)}</dd>
          <dt>Restart</dt>
          <dd>{describe(query.restart)}</dd>
        </dl>
      )}
    </div>
  );
}

function describe(control: EpicPauseRestartQuery['pause']): string {
  if (!control.current) return control.reason;
  const { status, launchedCount, targetCount } = control.current;
  return `${control.reason} ${status}: ${launchedCount} of ${targetCount} dispatches launch-accepted. Provider receipt, compliance, and progress are not observed.`;
}
