import { useState } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type { WorkUnitAgentSessionPresentation } from '../orchestrationModel';
import { DetailWorkspace } from './DetailWorkspace';
import { ResizableSplitSurface } from './ResizableSplitSurface';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';

export interface WorkUnitDetailWorkspaceProps {
  readonly unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];
  readonly lifecycleEntries: SprintWorkspacePresentationV1['workUnitLifecycle'];
  readonly sprintPlannerActivityGroupTitle: string;
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly backLabel?: string;
  readonly onBack: () => void;
}

interface SessionFocusTarget {
  readonly sessionId: string;
  readonly invocationId: string;
  readonly request: number;
}

export function WorkUnitDetailWorkspace({
  unit,
  lifecycleEntries,
  sprintPlannerActivityGroupTitle,
  sessions,
  agentSessionComposition,
  backLabel = 'Back to Plan',
  onBack,
}: WorkUnitDetailWorkspaceProps) {
  const workUnitId = unit.workUnitId;
  const handler = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'handler',
  );
  const worker = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'worker',
  );
  const reviewer = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'reviewer',
  );
  const [secondarySessionId, setSecondarySessionId] = useState(
    worker?.sessionId ?? reviewer?.sessionId ?? '',
  );
  const [focusTarget, setFocusTarget] = useState<SessionFocusTarget | null>(null);
  const secondarySession =
    sessions.find(({ sessionId }) => sessionId === secondarySessionId) ?? worker ?? reviewer;

  const navigateToLifecycleTurn = (
    entry: SprintWorkspacePresentationV1['workUnitLifecycle'][number],
  ) => {
    if (entry.agentSessionId !== handler?.sessionId) setSecondarySessionId(entry.agentSessionId);
    setFocusTarget((current) => ({
      sessionId: entry.agentSessionId,
      invocationId: entry.invocationId,
      request: (current?.request ?? 0) + 1,
    }));
  };

  return (
    <DetailWorkspace
      ariaLabel={`Work Unit detail: ${workUnitId}`}
      controlsLabel="Work Unit controls"
      contextLabel="Work Unit context"
      backLabel={backLabel}
      onBack={onBack}
      focusBackOnMount
      hotbarContext={sprintPlannerActivityGroupTitle}
      control={
        <span className={`work-unit-state work-unit-state--${unit.presentationState}`}>
          {workUnitStatusLabel(unit.presentationState)}
        </span>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Work Unit</p>
          <code>{unit.workUnitId}</code>
          <h1>{unit.title}</h1>
          <p>{unit.summary}</p>
          <p>{unit.details}</p>
          <p className="work-unit-fixture-notice">
            Recorded/theoretical fixture only. No live execution or persistence.
          </p>
          <section className="work-unit-lifecycle" aria-label="Work Unit lifecycle turn log">
            <h2>Lifecycle</h2>
            {lifecycleEntries.length ? (
              <ol>
                {lifecycleEntries.map((entry) => {
                  const session = sessions.find(
                    ({ sessionId }) => sessionId === entry.agentSessionId,
                  );
                  return (
                    <li key={entry.entryId}>
                      <button
                        type="button"
                        onClick={() => navigateToLifecycleTurn(entry)}
                        disabled={!session}
                      >
                        <span
                          className={`work-unit-lifecycle__identity work-unit-lifecycle__identity--${entry.agentRole}`}
                          aria-hidden="true"
                        >
                          {agentInitial(entry.agentRole)}
                        </span>
                        <span>
                          <strong>{entry.title}</strong>
                          <small>{session?.title ?? 'Recorded Agent Session unavailable'}</small>
                          <span>{entry.summary}</span>
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ol>
            ) : (
              <p>No recorded lifecycle turn links are available for this Work Unit.</p>
            )}
          </section>
        </div>
      }
      primary={
        <section className="work-unit-sessions" aria-label="Work Unit Agent Sessions">
          <ResizableSplitSurface
            axis="horizontal"
            primary={
              <SessionSlot
                label="Handler / planner fork"
                session={handler}
                agentSessionComposition={agentSessionComposition}
                focusTarget={focusTarget}
              />
            }
            secondary={
              <div className="work-unit-execution-session">
                {worker && reviewer ? (
                  <nav aria-label="Execution Agent Session">
                    {[worker, reviewer].map((session) => (
                      <button
                        key={session.sessionId}
                        type="button"
                        aria-pressed={secondarySession?.sessionId === session.sessionId}
                        onClick={() => setSecondarySessionId(session.sessionId)}
                      >
                        {session.role === 'worker' ? 'Worker' : 'Reviewer'}
                      </button>
                    ))}
                  </nav>
                ) : null}
                <SessionSlot
                  label={
                    secondarySession?.role === 'reviewer' ? 'Reviewer' : 'Implementation worker'
                  }
                  session={secondarySession}
                  agentSessionComposition={agentSessionComposition}
                  focusTarget={focusTarget}
                />
              </div>
            }
            primaryLabel="Handler conversation"
            secondaryLabel="Work and review conversation"
            initialPrimaryPercent={50}
            minimumPrimaryPixels={220}
            minimumSecondaryPixels={220}
          />
        </section>
      }
    />
  );
}

function SessionSlot({
  label,
  session,
  agentSessionComposition,
  focusTarget,
}: {
  readonly label: string;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly focusTarget: SessionFocusTarget | null;
}) {
  return (
    <div className="work-unit-session-slot">
      <h2>{label}</h2>
      {session ? (
        <SharedAgentSessionPanel
          ariaLabel={`${label} Agent Session`}
          conversationAriaLabel={`${label} conversation`}
          session={session}
          composition={agentSessionComposition}
          displayMode="always_open"
          focusInvocationId={
            focusTarget?.sessionId === session.sessionId ? focusTarget.invocationId : undefined
          }
          focusRequest={focusTarget?.request}
        />
      ) : (
        <section className="work-unit-session-empty" aria-label={`${label} unavailable`}>
          <strong>No recorded session</strong>
          <p>This projected Work Unit has no manufactured launch or conversation.</p>
        </section>
      )}
    </div>
  );
}

function agentInitial(
  role: SprintWorkspacePresentationV1['workUnitLifecycle'][number]['agentRole'],
) {
  return { planner: 'P', worker: 'W', reviewer: 'R', merger: 'M' }[role];
}

function workUnitStatusLabel(
  state: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['presentationState'],
) {
  if (['integrated', 'responsibility_accepted'].includes(state)) return 'Completed';
  if (['requested', 'launched', 'returned', 'under_review'].includes(state)) return 'Processing';
  return state === 'deferred' ? 'Deferred' : 'Planned';
}
