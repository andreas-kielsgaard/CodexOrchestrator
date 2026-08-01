import { useState } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { WorkUnitAgentSessionPresentation } from '../orchestrationModel';
import { DetailWorkspace } from './DetailWorkspace';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';

export interface WorkUnitDetailWorkspaceProps {
  readonly unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];
  readonly sprintPlannerActivityGroupTitle: string;
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly backLabel?: string;
  readonly onBack: () => void;
}

export function WorkUnitDetailWorkspace({
  unit,
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
  const implementer = sessions.find(
    (session) => session.workUnitId === workUnitId && session.role === 'implementer',
  );
  const [dominant, setDominant] = useState<'handler' | 'implementer' | null>(null);

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
          {unit.presentationState.replaceAll('_', ' ')}
        </span>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Work Unit</p>
          <code>{unit.workUnitId}</code>
          <h1>{unit.title}</h1>
          <p>{unit.summary}</p>
          <p className="work-unit-fixture-notice">
            Recorded/theoretical fixture only. No live execution or persistence.
          </p>
          <dl>
            <div>
              <dt>Direction</dt>
              <dd>{unit.details}</dd>
            </div>
            <div>
              <dt>Attempts</dt>
              <dd>{unit.attempts.length}</dd>
            </div>
            {unit.attempts.map((attempt) => {
              const review = unit.reviews.find(({ attemptId }) => attemptId === attempt.attemptId);
              return (
                <div key={attempt.attemptId}>
                  <dt>{attempt.attemptId}</dt>
                  <dd>
                    {attempt.returned ? 'Returned' : 'Not returned'}
                    {review?.outcome ? ` · ${review.outcome.replaceAll('_', ' ')}` : ''}
                  </dd>
                </div>
              );
            })}
          </dl>
        </div>
      }
      primary={
        <section
          className="work-unit-sessions"
          aria-label="Handler and Implementer Agent Sessions"
          data-dominant={dominant ?? 'equal'}
        >
          <SessionSlot
            label="Handler / planner fork"
            session={handler}
            agentSessionComposition={agentSessionComposition}
            expanded={dominant !== 'implementer'}
            onExpandedChange={(expanded) => setDominant(expanded ? 'handler' : 'implementer')}
          />
          <SessionSlot
            label="Work Unit Implementer"
            session={implementer}
            agentSessionComposition={agentSessionComposition}
            expanded={dominant !== 'handler'}
            onExpandedChange={(expanded) => setDominant(expanded ? 'implementer' : 'handler')}
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
  expanded,
  onExpandedChange,
}: {
  readonly label: string;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly expanded: boolean;
  readonly onExpandedChange: (expanded: boolean) => void;
}) {
  return (
    <div className="work-unit-session-slot" data-expanded={expanded}>
      <h2>{label}</h2>
      {session ? (
        <SharedAgentSessionPanel
          ariaLabel={`${label} Agent Session`}
          conversationAriaLabel={`${label} conversation`}
          session={session}
          composition={agentSessionComposition}
          expanded={expanded}
          onExpandedChange={onExpandedChange}
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
