import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../../application/orchestrations/recordedPlanWorkflow';
import { DetailWorkspace } from './DetailWorkspace';
import { PlanWorkflowMap } from './PlanWorkflowMap';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type { SprintAgentSessionPresentation } from '../orchestrationModel';
import { ResizableSplitSurface } from './ResizableSplitSurface';
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
              {sessions.length === 2 ? (
                <div className="plan-agent-session-split">
                  <ResizableSplitSurface
                    axis="horizontal"
                    primaryLabel={`${sessions[0].title} conversation`}
                    secondaryLabel={`${sessions[1].title} conversation`}
                    initialPrimaryPercent={50}
                    minimumPrimaryPixels={280}
                    minimumSecondaryPixels={280}
                    primary={
                      <SharedAgentSessionPanel
                        ariaLabel={`${sessions[0].title} Agent Session`}
                        conversationAriaLabel={`${sessions[0].title} conversation`}
                        session={sessions[0]}
                        composition={agentSessionComposition}
                        displayMode="always_open"
                      />
                    }
                    secondary={
                      <SharedAgentSessionPanel
                        ariaLabel={`${sessions[1].title} Agent Session`}
                        conversationAriaLabel={`${sessions[1].title} conversation`}
                        session={sessions[1]}
                        composition={agentSessionComposition}
                        displayMode="always_open"
                      />
                    }
                  />
                </div>
              ) : (
                sessions.map((session) => (
                  <SharedAgentSessionPanel
                    key={session.sessionId}
                    ariaLabel={`${session.title} Agent Session`}
                    conversationAriaLabel={`${session.title} conversation`}
                    session={session}
                    composition={agentSessionComposition}
                  />
                ))
              )}
            </section>
          )}
        </>
      }
    />
  );
}
