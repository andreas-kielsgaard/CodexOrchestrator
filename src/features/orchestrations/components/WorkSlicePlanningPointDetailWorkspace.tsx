import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { RecordedPlanWorkflowV1 } from '../../../application/orchestrations/recordedPlanWorkflow';
import { DetailWorkspace } from './DetailWorkspace';
import { PlanWorkflowMap } from './PlanWorkflowMap';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  SprintAgentSessionPresentation,
  WorkUnitAgentSessionPresentation,
} from '../orchestrationModel';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';
import type { ReactNode } from 'react';

type RevisionWorkUnit = SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];

export interface PlanningPointWorkUnitRelationship {
  readonly workUnit: RevisionWorkUnit;
  readonly handlers: readonly WorkUnitAgentSessionPresentation[];
  readonly implementers: readonly WorkUnitAgentSessionPresentation[];
}

export interface WorkSlicePlanningPointDetailWorkspaceProps {
  readonly workSlicePlanningPointGroup: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number];
  readonly currentWorkState: string;
  readonly workUnitRelationships: readonly PlanningPointWorkUnitRelationship[];
  readonly workflow?: RecordedPlanWorkflowV1;
  readonly plannerSession?: SprintAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onBack: () => void;
  readonly onOpenWorkUnit: (workUnitId: string) => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly sprintControl?: ReactNode;
}

export function WorkSlicePlanningPointDetailWorkspace({
  workSlicePlanningPointGroup,
  currentWorkState,
  workUnitRelationships,
  workflow,
  plannerSession,
  agentSessionComposition,
  onBack,
  onOpenWorkUnit,
  onOpenAgentSession,
  sprintControl,
}: WorkSlicePlanningPointDetailWorkspaceProps) {
  return (
    <DetailWorkspace
      ariaLabel={`Work Slice planning point detail: ${workSlicePlanningPointGroup.title}`}
      controlsLabel="Work Slice planning point controls"
      contextLabel="Work Slice planning point context"
      backLabel="Back to Sprint"
      onBack={onBack}
      focusBackOnMount
      control={
        <div className="sprint-header-controls">
          {sprintControl}
          <span className="current-work-state">
            <small>Current work</small>
            <strong>{currentWorkState}</strong>
          </span>
        </div>
      }
      context={
        <div className="subdetail-context">
          <p className="eyebrow">Work Slice planning point</p>
          <h1>{workSlicePlanningPointGroup.title}</h1>
          <p>{workSlicePlanningPointGroup.purpose}</p>
        </div>
      }
      primary={
        <div className="work-slice-planning-point-detail">
          <PlanningPointRelationships
            plannerSession={plannerSession}
            workUnitRelationships={workUnitRelationships}
            onOpenWorkUnit={onOpenWorkUnit}
          />
          {workflow ? (
            <section
              className="work-slice-planning-point-detail__workflow"
              aria-label="Detailed recorded workflow"
            >
              <h2>Detailed recorded workflow</h2>
              <PlanWorkflowMap workflow={workflow} />
            </section>
          ) : (
            <section className="plan-workflow-empty" aria-label="Detailed workflow unavailable">
              <strong>Detailed workflow unavailable.</strong>
              <p>No detailed turn sequence is recorded for this Work Slice planning point.</p>
            </section>
          )}
          {plannerSession && (
            <section className="plan-agent-sessions" aria-label="Work Slice Planner Agent Session">
              <SharedAgentSessionPanel
                ariaLabel="Work Slice Planner Agent Session conversation surface"
                conversationAriaLabel="Work Slice Planner conversation"
                session={plannerSession}
                composition={agentSessionComposition}
                onOpenStandalone={onOpenAgentSession}
              />
            </section>
          )}
        </div>
      }
    />
  );
}

function PlanningPointRelationships({
  plannerSession,
  workUnitRelationships,
  onOpenWorkUnit,
}: {
  readonly plannerSession?: SprintAgentSessionPresentation;
  readonly workUnitRelationships: readonly PlanningPointWorkUnitRelationship[];
  readonly onOpenWorkUnit: (workUnitId: string) => void;
}) {
  return (
    <section
      className="planning-point-relationships"
      aria-label="Work Slice planning point relationships"
    >
      <div className="planning-point-relationships__flow">
        <section
          className="planning-point-relationships__planner"
          aria-label="Work Slice Planner relationship"
        >
          <span>Work Slice Planner</span>
          {plannerSession ? (
            <strong>{plannerSession.title}</strong>
          ) : (
            <p aria-label="Work Slice Planner unavailable">
              <strong>Unavailable</strong>
              <span>No typed Work Slice Planner Agent Session relationship is recorded.</span>
            </p>
          )}
        </section>
        <span className="planning-point-relationships__arrow" aria-hidden="true">
          →
        </span>
        {workUnitRelationships.length ? (
          <div
            className="planning-point-relationships__work-units"
            role="list"
            aria-label="Work Units scoped to this Work Slice planning point"
          >
            {workUnitRelationships.map(({ workUnit, handlers, implementers }) => (
              <article
                key={workUnit.workUnitId}
                className="planning-point-work-unit"
                role="listitem"
                aria-label={`Work Unit ${workUnit.workUnitId}: ${workUnit.title}`}
              >
                <header>
                  <span>Work Unit</span>
                  <small>{workUnitStatusLabel(workUnit.presentationState)}</small>
                </header>
                <code>{workUnit.workUnitId}</code>
                <h2>{workUnit.title}</h2>
                <button type="button" onClick={() => onOpenWorkUnit(workUnit.workUnitId)}>
                  Open Work Unit {workUnit.workUnitId}: {workUnit.title}
                </button>
                <AgentRelationships
                  workUnitId={workUnit.workUnitId}
                  label="Work Unit Handler"
                  sessions={handlers}
                />
                <AgentRelationships
                  workUnitId={workUnit.workUnitId}
                  label="Work Unit Implementer"
                  sessions={implementers}
                />
              </article>
            ))}
          </div>
        ) : (
          <p className="planning-point-relationships__empty">No scoped Work Units are recorded.</p>
        )}
      </div>
    </section>
  );
}

function AgentRelationships({
  workUnitId,
  label,
  sessions,
}: {
  readonly workUnitId: string;
  readonly label: 'Work Unit Handler' | 'Work Unit Implementer';
  readonly sessions: readonly WorkUnitAgentSessionPresentation[];
}) {
  return (
    <section
      className="planning-point-work-unit__agent-relationship"
      aria-label={`${workUnitId} ${label} relationship`}
    >
      <h3>{label}</h3>
      {sessions.length ? (
        <ul>
          {sessions.map((session) => (
            <li key={session.sessionId}>{session.title}</li>
          ))}
        </ul>
      ) : (
        <p aria-label={`${workUnitId} ${label} unavailable`}>
          <strong>Unavailable</strong>
          <span>No typed Agent Session relationship is recorded.</span>
        </p>
      )}
    </section>
  );
}

function workUnitStatusLabel(state: RevisionWorkUnit['presentationState']) {
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
