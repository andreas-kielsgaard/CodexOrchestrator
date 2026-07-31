import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../../application/orchestrations/recordedPlanWorkflow';
import { DetailWorkspace } from './DetailWorkspace';
import { PlanWorkflowMap } from './PlanWorkflowMap';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type { SprintAgentSessionPresentation } from '../orchestrationModel';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';

export interface WorkSlicePlanningPointDetailWorkspaceProps {
  readonly workSlicePlanningPointGroup: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number];
  readonly currentWorkState: string;
  readonly workflow?: RecordedPlanWorkflowV1;
  readonly session?: SprintAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onBack: () => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
}

export function WorkSlicePlanningPointDetailWorkspace({
  workSlicePlanningPointGroup,
  currentWorkState,
  workflow,
  session,
  agentSessionComposition,
  onBack,
  onOpenAgentSession,
}: WorkSlicePlanningPointDetailWorkspaceProps) {
  return (
    <DetailWorkspace
      ariaLabel={`Plan detail: ${workSlicePlanningPointGroup.title}`}
      controlsLabel="Plan controls"
      contextLabel="Plan context"
      backLabel="Back to Sprint"
      onBack={onBack}
      focusBackOnMount
      control={
        <span className="current-work-state">
          <small>Current work</small>
          <strong>{currentWorkState}</strong>
        </span>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Plan</p>
          <h1>{workSlicePlanningPointGroup.title}</h1>
          <p>{workSlicePlanningPointGroup.purpose}</p>
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
          {session && (
            <section className="plan-agent-sessions" aria-label="Plan Agent Sessions">
              <SharedAgentSessionPanel
                ariaLabel={`${session.title} Agent Session`}
                conversationAriaLabel={`${session.title} conversation`}
                session={session}
                composition={agentSessionComposition}
                onOpenStandalone={onOpenAgentSession}
              />
            </section>
          )}
        </>
      }
    />
  );
}
