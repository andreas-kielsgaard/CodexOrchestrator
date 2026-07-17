import {
  Bot,
  CheckCheck,
  GitMerge,
  MessageSquareReply,
  Network,
  RotateCcw,
  SearchCheck,
  Send,
  Workflow,
} from 'lucide-react';
import type { CSSProperties } from 'react';
import type {
  PlanWorkflowStepV1,
  RecordedPlanWorkflowV1,
} from '../../../application/orchestrations/recordedPlanWorkflow';

export function PlanWorkflowMap({ workflow }: { readonly workflow: RecordedPlanWorkflowV1 }) {
  const actors = new Map(workflow.actors.map((actor) => [actor.id, actor]));
  const correction = workflow.interactions.find(({ kind }) => kind === 'correction_loop');
  const sameWorker = correction?.sameActorId ? actors.get(correction.sameActorId) : undefined;

  return (
    <section className="plan-workflow" aria-label="Plan actor and conversation workflow">
      <header className="plan-workflow__notice">
        <Workflow size={17} aria-hidden="true" />
        <span>
          <strong>Recorded/theoretical evaluation</strong>
          <small>No live execution, thread creation, repository integration, or persistence.</small>
        </span>
      </header>

      <div className="plan-workflow__canvas">
        <WorkflowTrack label="Sprint to Planner" steps={workflow.sharedStart} actors={actors} />

        <section className="plan-workflow__parallel" aria-label="Parallel Work Unit lanes">
          <header>
            <Network size={16} aria-hidden="true" />
            <span>Planner creates one initiator per ready Work Unit</span>
            <strong>{workflow.workUnitLanes.length} parallel lanes</strong>
          </header>
          {workflow.workUnitLanes.map((lane) => (
            <article
              key={lane.id}
              className="plan-workflow-lane"
              aria-label={`Work Unit lane: ${lane.title}`}
              data-work-unit-lane={lane.id}
            >
              <div className="plan-workflow-lane__label">
                <span>Work Unit</span>
                <strong>{lane.title}</strong>
                <code>{lane.workUnitId}</code>
              </div>
              <div className="plan-workflow-lane__track">
                {lane.steps.map((step) => (
                  <WorkflowNode
                    key={step.id}
                    step={step}
                    actorLabel={actors.get(step.actorId)?.label}
                  />
                ))}
                {correction && lane.steps.some(({ id }) => id === correction.fromStepId) && (
                  <div
                    className="plan-workflow__correction-loop"
                    data-correction-loop={correction.id}
                    data-same-worker={correction.sameActorId}
                  >
                    <RotateCcw size={14} aria-hidden="true" />
                    <span>Re-prompt the same worker thread</span>
                    <strong>{sameWorker?.label}</strong>
                  </div>
                )}
              </div>
            </article>
          ))}
        </section>

        <WorkflowTrack
          label="All Work Units settled"
          steps={workflow.sharedCompletion}
          actors={actors}
          completion
        />
      </div>
    </section>
  );
}

function WorkflowTrack({
  label,
  steps,
  actors,
  completion = false,
}: {
  readonly label: string;
  readonly steps: readonly PlanWorkflowStepV1[];
  readonly actors: ReadonlyMap<string, RecordedPlanWorkflowV1['actors'][number]>;
  readonly completion?: boolean;
}) {
  return (
    <section
      className={`plan-workflow__shared${completion ? ' is-completion' : ''}`}
      aria-label={label}
    >
      <span className="plan-workflow__shared-label">{label}</span>
      <div className="plan-workflow__shared-track">
        {steps.map((step) => (
          <WorkflowNode key={step.id} step={step} actorLabel={actors.get(step.actorId)?.label} />
        ))}
      </div>
    </section>
  );
}

function WorkflowNode({
  step,
  actorLabel,
}: {
  readonly step: PlanWorkflowStepV1;
  readonly actorLabel?: string;
}) {
  const Icon = iconFor(step.kind);
  return (
    <div
      className={`plan-workflow-node plan-workflow-node--${step.kind}`}
      data-workflow-step={step.id}
      style={{ '--workflow-column': phaseColumn(step.phase) } as CSSProperties}
    >
      <span className="plan-workflow-node__icon">
        <Icon size={16} aria-hidden="true" />
      </span>
      <span className="plan-workflow-node__content">
        <small>{actorLabel}</small>
        <strong>{step.title}</strong>
        {step.cycle && <em>Review {step.cycle}</em>}
      </span>
    </div>
  );
}

function phaseColumn(phase: PlanWorkflowStepV1['phase']): number {
  return {
    ready: 1,
    planner_start: 2,
    scope: 3,
    work_unit_start: 1,
    worker_start: 2,
    first_return: 3,
    first_review: 4,
    correction: 5,
    second_return: 6,
    second_review: 7,
    integration: 8,
    settled: 9,
    planner_complete: 1,
    sprint_return: 2,
  }[phase];
}

function iconFor(kind: PlanWorkflowStepV1['kind']) {
  if (kind.includes('worker')) return Bot;
  if (kind.includes('review')) return SearchCheck;
  if (kind.includes('return')) return MessageSquareReply;
  if (kind.includes('integration') || kind === 'handoff') return GitMerge;
  if (kind.includes('settled') || kind.includes('completed')) return CheckCheck;
  if (kind.includes('initiator')) return Send;
  if (kind === 'correction_required') return RotateCcw;
  return Workflow;
}
