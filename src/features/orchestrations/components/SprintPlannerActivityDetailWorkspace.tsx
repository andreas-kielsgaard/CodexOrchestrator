import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../../application/orchestrations/recordedPlanWorkflow';
import { DetailWorkspace } from './DetailWorkspace';
import { PlanWorkflowMap } from './PlanWorkflowMap';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type { SprintAgentSessionPresentation } from '../orchestrationModel';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';

export interface SprintPlannerActivityDetailWorkspaceProps {
  readonly plannerActivityGroup: SprintWorkspacePresentationV1['revisionViews'][number]['plannerActivityGroups'][number];
  readonly workflow?: RecordedPlanWorkflowV1;
  readonly sessions: readonly SprintAgentSessionPresentation[];
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onBack: () => void;
}

export function SprintPlannerActivityDetailWorkspace({
  plannerActivityGroup,
  workflow,
  sessions,
  agentSessionComposition,
  onBack,
}: SprintPlannerActivityDetailWorkspaceProps) {
  return (
    <DetailWorkspace
      ariaLabel={`Plan detail: ${plannerActivityGroup.title}`}
      controlsLabel="Plan controls"
      contextLabel="Plan context"
      backLabel="Back to Sprint"
      onBack={onBack}
      focusBackOnMount
      control={<span className="subdetail-count">Fixed ready scope</span>}
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Plan</p>
          <h1>{plannerActivityGroup.title}</h1>
          <p>{plannerActivityGroup.purpose}</p>
        </div>
      }
      primary={
        <>
          {workflow ? (
            <PlanWorkflowMap workflow={workflow} />
          ) : (
            <section className="plan-workflow-empty" aria-label="Plan workflow unavailable">
              <strong>No recorded workflow for this historical Plan.</strong>
              <p>The view does not manufacture actors, launches, or conversations.</p>
            </section>
          )}
          {sessions.length > 0 && (
            <section className="plan-agent-sessions" aria-label="Plan Agent Sessions">
              {sessions.map((session) => (
                <SharedAgentSessionPanel
                  key={session.sessionId}
                  ariaLabel={`${session.title} Agent Session`}
                  conversationAriaLabel={`${session.title} conversation`}
                  session={session}
                  composition={agentSessionComposition}
                />
              ))}
            </section>
          )}
        </>
      }
    />
  );
}
