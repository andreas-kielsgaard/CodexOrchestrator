import { useEffect, useState } from 'react';
import type { EpicPauseRestartController, EpicPauseRestartQuery } from '../../../application/orchestrations';

export function EpicPauseRestartControl({ epicId, controller }: { readonly epicId: string; readonly controller?: EpicPauseRestartController }) {
  const [outcome, setOutcome] = useState('');
  const [query, setQuery] = useState<EpicPauseRestartQuery | null>(null);
  const refresh = async () => {
    if (!controller) return;
    try { setQuery(await controller.load(epicId)); } catch { setOutcome('Epic controls are unavailable because durable control state could not be loaded.'); }
  };
  useEffect(() => { void refresh(); }, [controller, epicId]);
  const request = async (kind: 'pause' | 'restart') => {
    if (!controller) return setOutcome('Epic controls are unavailable.');
    try {
      const result = kind === 'pause' ? await controller.requestPause(epicId) : await controller.requestRestart(epicId);
      setOutcome(`${result.kind === 'pause' ? 'Pause' : 'Restart'} ${result.status}: ${result.launchedCount} of ${result.targetCount} dispatches launch-accepted. Provider receipt, compliance, and progress are not observed.`);
      await refresh();
    } catch { setOutcome('The durable Epic control request is unavailable.'); }
  };
  return (
    <div className="epic-pause-restart-control">
      <button type="button" disabled={!query || query.pause.availability !== 'available'} onClick={() => void request('pause')}>
        Pause
      </button>
      <button type="button" disabled={!query || query.restart.availability !== 'available'} onClick={() => void request('restart')}>
        Restart
      </button>
      <p role="status" aria-live="polite">
        {outcome || query?.pause.reason || 'Loading durable Epic controls.'}
      </p>
    </div>
  );
}
