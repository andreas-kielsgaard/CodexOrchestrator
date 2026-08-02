import { useEffect, useState } from 'react';
import type {
  EpicPauseRestartController,
  EpicPauseRestartQuery,
} from '../../../application/orchestrations';

export function EpicPauseRestartControl({
  epicId,
  controller,
}: {
  readonly epicId: string;
  readonly controller?: EpicPauseRestartController;
}) {
  const [outcome, setOutcome] = useState('');
  const [query, setQuery] = useState<EpicPauseRestartQuery | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const refresh = async () => {
    if (!controller) return;
    try {
      setQuery(await controller.load(epicId));
    } catch {
      setOutcome(
        'Epic controls are unavailable because durable control state could not be loaded.',
      );
    }
  };
  useEffect(() => {
    void refresh();
  }, [controller, epicId]);
  const request = async (kind: 'pause' | 'restart') => {
    if (!controller || submitting) return;
    setSubmitting(true);
    try {
      const result =
        kind === 'pause'
          ? await controller.requestPause(epicId)
          : await controller.requestRestart(epicId);
      setOutcome(
        `${result.kind === 'pause' ? 'Pause' : 'Restart'} ${result.status}: ${result.launchedCount} of ${result.targetCount} dispatches launch-accepted. Provider receipt, compliance, and progress are not observed.`,
      );
      await refresh();
    } catch {
      setOutcome('The durable Epic control request is unavailable.');
    } finally {
      setSubmitting(false);
    }
  };
  return (
    <div className="epic-pause-restart-control">
      <button
        type="button"
        disabled={submitting || !query || query.pause.availability !== 'available'}
        onClick={() => void request('pause')}
      >
        Pause
      </button>
      <button
        type="button"
        disabled={submitting || !query || query.restart.availability !== 'available'}
        onClick={() => void request('restart')}
      >
        Restart
      </button>
      <p role="status" aria-live="polite">
        {outcome || 'Loading durable Epic controls.'}
      </p>
      {query && (
        <>
          <dl>
            <dt>Pause</dt>
            <dd>{describe(query.pause)}</dd>
            <dt>Restart</dt>
            <dd>{describe(query.restart)}</dd>
          </dl>
          <ControlEvidence title="Pause evidence" targets={query.pause.current?.targets ?? []} />
          <ControlEvidence
            title="Restart evidence"
            targets={query.restart.current?.targets ?? []}
          />
        </>
      )}
    </div>
  );
}

function describe(control: EpicPauseRestartQuery['pause']): string {
  if (!control.current) return control.reason;
  const { status, launchedCount, targetCount } = control.current;
  return `${control.reason} ${status}: ${launchedCount} of ${targetCount} dispatches launch-accepted. Provider receipt, compliance, and progress are not observed.`;
}

function ControlEvidence({
  title,
  targets,
}: {
  readonly title: string;
  readonly targets: ReadonlyArray<
    NonNullable<EpicPauseRestartQuery['pause']['current']>['targets'][number]
  >;
}) {
  if (targets.length === 0) return null;
  return (
    <section aria-label={title}>
      <h4>{title}</h4>
      <ul>
        {targets.map((target) => (
          <li key={`${target.sessionId}:${target.sourceInvocationId}`}>{describeTarget(target)}</li>
        ))}
      </ul>
    </section>
  );
}

function describeTarget(
  target: NonNullable<EpicPauseRestartQuery['pause']['current']>['targets'][number],
): string {
  const source = target.sourceObservation
    ? `Source ${target.sourceInvocationId}: ${target.sourceObservation.providerActivity ? 'provider activity observed' : 'no provider activity observed'}; ${target.sourceObservation.processTerminal ? `process ${target.sourceObservation.processTerminal.status} observed` : 'no process terminal observed'}.`
    : `Source ${target.sourceInvocationId}: durable invocation observation is unavailable.`;
  const control = target.controlInvocation
    ? ` Application control invocation ${target.controlInvocation.invocationId} was persisted${target.controlInvocation.launchAcceptedAt ? ' and launch-accepted' : ' but is not launch-accepted'}; ${target.controlInvocation.observation?.providerActivity ? 'provider activity observed' : 'no provider activity observed'}.`
    : ' No application control invocation has been persisted.';
  const cancellation = target.cancelRequestedAt
    ? 'Cancellation was requested.'
    : 'Cancellation was not requested.';
  return `${cancellation} Interruption status: ${target.interruptionStatus}. ${source}${control}${target.failure ? ` Control attention: ${target.failure.category}.` : ''} Provider receipt, instruction compliance, and useful progress are not observed.`;
}
