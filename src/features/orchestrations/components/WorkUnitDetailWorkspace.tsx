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
  readonly onOpenAgentSession?: (sessionId: string) => void;
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
  onOpenAgentSession,
}: WorkUnitDetailWorkspaceProps) {
  const workUnitId = unit.workUnitId;
  const sprintPlanner = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'sprint_planner',
  );
  const handler = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'handler',
  );
  const worker = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'worker',
  );
  const reviewer = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'reviewer',
  );
  const [primarySessionId, setPrimarySessionId] = useState(
    handler?.sessionId ?? sprintPlanner?.sessionId ?? '',
  );
  const [secondarySessionId, setSecondarySessionId] = useState(
    worker?.sessionId ?? reviewer?.sessionId ?? '',
  );
  const [focusTarget, setFocusTarget] = useState<SessionFocusTarget | null>(null);
  const primarySession =
    sessions.find(({ sessionId }) => sessionId === primarySessionId) ?? handler ?? sprintPlanner;
  const secondarySession =
    sessions.find(({ sessionId }) => sessionId === secondarySessionId) ?? worker ?? reviewer;

  const navigateToLifecycleTurn = (
    entry: SprintWorkspacePresentationV1['workUnitLifecycle'][number],
  ) => {
    if ([handler?.sessionId, sprintPlanner?.sessionId].includes(entry.agentSessionId))
      setPrimarySessionId(entry.agentSessionId);
    else setSecondarySessionId(entry.agentSessionId);
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
          <small>Current work</small>
          <strong>{workUnitStatusLabel(unit.presentationState)}</strong>
        </span>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Work Unit</p>
          <code>{unit.workUnitId}</code>
          <h1>{unit.title}</h1>
          <p>{unit.summary}</p>
          <p>{unit.details}</p>
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
              <div className="work-unit-primary-session">
                {sprintPlanner && handler ? (
                  <nav aria-label="Planning and handling Agent Session">
                    {[sprintPlanner, handler].map((session) => (
                      <button
                        key={session.sessionId}
                        type="button"
                        aria-pressed={primarySession?.sessionId === session.sessionId}
                        onClick={() => setPrimarySessionId(session.sessionId)}
                      >
                        {session.role === 'sprint_planner' ? 'Sprint Planner' : 'Work Unit handler'}
                      </button>
                    ))}
                  </nav>
                ) : null}
                <SessionSlot
                  label={
                    primarySession?.role === 'sprint_planner'
                      ? 'Sprint Planner'
                      : 'Work Unit handler'
                  }
                  session={primarySession}
                  agentSessionComposition={agentSessionComposition}
                  focusTarget={focusTarget}
                  onOpenAgentSession={onOpenAgentSession}
                />
              </div>
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
                  onOpenAgentSession={onOpenAgentSession}
                />
              </div>
            }
            primaryLabel="Planning and handling conversation"
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
  onOpenAgentSession,
  focusTarget,
}: {
  readonly label: string;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onOpenAgentSession?: (sessionId: string) => void;
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
          onOpenStandalone={onOpenAgentSession}
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
  return {
    sprint_planner: 'SP',
    work_unit_handler: 'H',
    worker: 'W',
    reviewer: 'R',
    merger: 'M',
  }[role];
}

function workUnitStatusLabel(
  state: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['presentationState'],
) {
  return {
    not_started: 'Planned',
    waiting_for_dependencies: 'Waiting for dependencies',
    requested: 'Requested',
    launched: 'In progress',
    returned: 'Returned',
    under_review: 'Under review',
    integrated: 'Completed',
    responsibility_accepted: 'Completed',
    deferred: 'Deferred',
  }[state];
}
