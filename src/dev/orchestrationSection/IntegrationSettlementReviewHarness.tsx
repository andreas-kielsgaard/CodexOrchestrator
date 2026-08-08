import { Fragment, useEffect, useRef, useState } from 'react';
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  Bot,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock3,
  ClipboardCheck,
  FileCheck2,
  FileCode2,
  GitBranch,
  Layers3,
  Link2,
  ShieldCheck,
  Wrench,
} from 'lucide-react';
import './integrationSettlementReviewHarness.css';

type ActivityKey =
  | 'implementer-claim'
  | 'application-evidence'
  | 'handler-review'
  | 'integration-applied'
  | 'settlement-confirmed'
  | 'prerequisite-contribution';

interface ActivityStage {
  readonly key: ActivityKey;
  readonly title: string;
  readonly status: string;
  readonly startedAt: string;
  readonly completedAt: string;
  readonly owner: string;
  readonly summary: string;
  readonly turnId: string;
  readonly icon: typeof ClipboardCheck;
}

interface SessionTurn {
  readonly id: string;
  readonly activityKey: ActivityKey;
  readonly kind: 'agent' | 'application' | 'mcp';
  readonly startedAt: string;
  readonly completedAt: string;
  readonly name: string;
  readonly role: 'Worker' | 'Handler' | 'Application' | 'MCP';
  readonly text: string;
  readonly fullOutput: string;
  readonly steps: readonly SessionStep[];
}

interface SessionStep {
  readonly id: string;
  readonly title: string;
  readonly summary: string;
}

const stages: readonly ActivityStage[] = [
  {
    key: 'implementer-claim',
    title: 'Implementer claim',
    status: 'Claimed',
    startedAt: '2026-08-05T09:54:00Z',
    completedAt: '2026-08-05T10:00:01Z',
    owner: 'Worker: Ada Lovelace',
    summary: 'Implemented the integration and collected evidence.',
    turnId: 'turn-implementation',
    icon: Bot,
  },
  {
    key: 'application-evidence',
    title: 'Application evidence',
    status: 'Collected',
    startedAt: '2026-08-05T10:00:01Z',
    completedAt: '2026-08-05T10:01:15Z',
    owner: 'Application',
    summary: 'Three independently addressable records were captured.',
    turnId: 'turn-evidence',
    icon: FileCheck2,
  },
  {
    key: 'handler-review',
    title: 'Handler review',
    status: 'Accepted',
    startedAt: '2026-08-05T10:01:15Z',
    completedAt: '2026-08-05T10:02:37Z',
    owner: 'Handler: Grace Hopper',
    summary: 'Review complete. Evidence supports acceptance.',
    turnId: 'turn-review',
    icon: ShieldCheck,
  },
  {
    key: 'integration-applied',
    title: 'Integration applied',
    status: 'Applied',
    startedAt: '2026-08-05T10:02:37Z',
    completedAt: '2026-08-05T10:03:12Z',
    owner: 'Application',
    summary: 'Accepted changes were applied to authoritative Sprint state.',
    turnId: 'turn-integration',
    icon: Layers3,
  },
  {
    key: 'settlement-confirmed',
    title: 'Settlement confirmed',
    status: 'Confirmed',
    startedAt: '2026-08-05T10:03:12Z',
    completedAt: '2026-08-05T10:03:48Z',
    owner: 'Handler: Grace Hopper',
    summary: 'The Work Unit no longer contributes active work.',
    turnId: 'turn-settlement',
    icon: CheckCircle2,
  },
  {
    key: 'prerequisite-contribution',
    title: 'Prerequisite contribution',
    status: 'Recorded',
    startedAt: '2026-08-05T10:03:48Z',
    completedAt: '2026-08-05T10:04:05Z',
    owner: 'Application',
    summary: 'Two dependent Work Units can now reevaluate readiness.',
    turnId: 'turn-prerequisite',
    icon: GitBranch,
  },
];

const applicationStageParents: Readonly<Partial<Record<ActivityKey, ActivityKey>>> = {
  'application-evidence': 'implementer-claim',
  'integration-applied': 'handler-review',
  'prerequisite-contribution': 'settlement-confirmed',
};

const turns: readonly SessionTurn[] = [
  {
    id: 'turn-implementation',
    activityKey: 'implementer-claim',
    kind: 'agent',
    startedAt: '2026-08-05T09:54:00Z',
    completedAt: '2026-08-05T10:00:01Z',
    name: 'Ada Lovelace',
    role: 'Worker',
    text: 'Implementation submitted; build and diff evidence attached.',
    fullOutput:
      'Implementation is ready for review. I applied the bounded integration change, kept the work within the assigned Work Unit, and attached the changed-file comparison and focused build result for independent inspection.',
    steps: [
      {
        id: 'implementation-scope',
        title: 'Confirmed the bounded scope',
        summary: 'Matched the requested Work Unit and isolated attempt before changing files.',
      },
      {
        id: 'implementation-change',
        title: 'Applied the implementation',
        summary: 'Updated the integration path without changing unrelated Sprint behavior.',
      },
      {
        id: 'implementation-validation',
        title: 'Collected validation evidence',
        summary: 'Recorded the changed-file comparison and focused build result.',
      },
    ],
  },
  {
    id: 'turn-evidence',
    activityKey: 'application-evidence',
    kind: 'application',
    startedAt: '2026-08-05T10:00:01Z',
    completedAt: '2026-08-05T10:01:15Z',
    name: 'Codex Orchestrator',
    role: 'Application',
    text: 'Application evidence is ready: diff, build result, and manifest references recorded.',
    fullOutput:
      'Application-owned evidence capture completed for the submitted attempt. The file comparison and build result are independently addressable. The integration manifest remains explicitly unavailable and is not treated as evidence.',
    steps: [
      {
        id: 'evidence-correlation',
        title: 'Correlated the submitted attempt',
        summary: 'Bound evidence capture to the exact Work Unit attempt and reporting turn.',
      },
      {
        id: 'evidence-capture',
        title: 'Captured available records',
        summary: 'Stored the file comparison and build result as application-owned records.',
      },
      {
        id: 'evidence-absence',
        title: 'Preserved unavailable evidence',
        summary: 'Kept the missing manifest visible as unavailable instead of inferring it.',
      },
    ],
  },
  {
    id: 'turn-review',
    activityKey: 'handler-review',
    kind: 'agent',
    startedAt: '2026-08-05T10:01:15Z',
    completedAt: '2026-08-05T10:02:37Z',
    name: 'Grace Hopper',
    role: 'Handler',
    text: 'Review complete. Evidence supports acceptance.',
    fullOutput:
      'Review complete. I inspected the application-owned file comparison and focused build result for this exact attempt. The change stays within the Work Unit scope, the available validation supports the stated outcome, and no conflicting evidence is present. Evidence supports acceptance for integration.',
    steps: [
      {
        id: 'review-scope',
        title: 'Verified review scope',
        summary: 'Confirmed that the evidence belongs to the selected Work Unit and attempt.',
      },
      {
        id: 'review-evidence',
        title: 'Inspected application evidence',
        summary: 'Reviewed the changed-file comparison and successful build record.',
      },
      {
        id: 'review-decision',
        title: 'Recorded the Handler judgment',
        summary: 'Accepted the implementation for integration without implying settlement.',
      },
    ],
  },
  {
    id: 'mcp-evidence-query',
    activityKey: 'application-evidence',
    kind: 'mcp',
    startedAt: '2026-08-05T10:00:42Z',
    completedAt: '2026-08-05T10:00:43Z',
    name: 'read_work_unit_evidence',
    role: 'MCP',
    text: 'Loaded the two available evidence records for this attempt.',
    fullOutput:
      'Called read_work_unit_evidence with the exact Work Unit and attempt identifiers. The response returned the changed-file comparison and build result, and preserved the integration manifest as unavailable.',
    steps: [
      {
        id: 'mcp-evidence-request',
        title: 'Sent tool request',
        summary: 'Arguments: workUnitId=unit-integration-review, attemptId=attempt-1.',
      },
      {
        id: 'mcp-evidence-response',
        title: 'Received tool response',
        summary: 'Two available records and one explicit unavailable record were returned.',
      },
    ],
  },
  {
    id: 'turn-integration',
    activityKey: 'integration-applied',
    kind: 'application',
    startedAt: '2026-08-05T10:02:37Z',
    completedAt: '2026-08-05T10:03:12Z',
    name: 'Codex Orchestrator',
    role: 'Application',
    text: 'Accepted candidate applied to authoritative Sprint state.',
    fullOutput:
      'The accepted candidate is authorized for application-owned integration. The integration will apply the verified change to the authoritative Sprint state while preserving the existing history and unrelated work.',
    steps: [
      {
        id: 'integration-authority',
        title: 'Confirmed integration authority',
        summary: 'Validated the accepted candidate and authoritative Sprint target.',
      },
      {
        id: 'integration-apply',
        title: 'Applied the accepted change',
        summary: 'Created the bounded integration effect against current Sprint state.',
      },
    ],
  },
  {
    id: 'turn-settlement',
    activityKey: 'settlement-confirmed',
    kind: 'agent',
    startedAt: '2026-08-05T10:03:12Z',
    completedAt: '2026-08-05T10:03:48Z',
    name: 'Grace Hopper',
    role: 'Handler',
    text: 'State settlement validated. This Work Unit is complete.',
    fullOutput:
      'The integration effect and authoritative Sprint state agree. This Work Unit is now settled. Settlement records completion of this responsibility; it does not itself start dependent work or settle the enclosing Work Slice.',
    steps: [
      {
        id: 'settlement-validate',
        title: 'Validated integrated state',
        summary: 'Confirmed the durable integration evidence and authoritative target state.',
      },
      {
        id: 'settlement-record',
        title: 'Recorded Work Unit settlement',
        summary: 'Closed this responsibility while preserving downstream boundaries.',
      },
    ],
  },
  {
    id: 'mcp-integration-record',
    activityKey: 'integration-applied',
    kind: 'mcp',
    startedAt: '2026-08-05T10:02:56Z',
    completedAt: '2026-08-05T10:02:58Z',
    name: 'apply_integration_candidate',
    role: 'MCP',
    text: 'Recorded the accepted candidate against authoritative Sprint state.',
    fullOutput:
      'Called apply_integration_candidate for the accepted revision and current Sprint version. The response returned one integration record and the resulting authoritative version identifier.',
    steps: [
      {
        id: 'mcp-integration-request',
        title: 'Sent tool request',
        summary: 'Arguments: revisionId=revision-integration-review, sprintVersion=42.',
      },
      {
        id: 'mcp-integration-response',
        title: 'Received tool response',
        summary: 'Integration record integration-381 and authoritative Sprint version 43 returned.',
      },
    ],
  },
  {
    id: 'turn-prerequisite',
    activityKey: 'prerequisite-contribution',
    kind: 'application',
    startedAt: '2026-08-05T10:03:48Z',
    completedAt: '2026-08-05T10:04:05Z',
    name: 'Codex Orchestrator',
    role: 'Application',
    text: 'Prerequisite contribution confirmed; dependent work can reevaluate readiness.',
    fullOutput:
      'The settled Work Unit now contributes its exact prerequisite result to two dependent Work Units. Those dependents may reevaluate readiness, but this contribution does not claim that either dependent has been activated or started.',
    steps: [
      {
        id: 'prerequisite-correlation',
        title: 'Matched dependent relationships',
        summary: 'Correlated the settled Work Unit to its two canonical dependent edges.',
      },
      {
        id: 'prerequisite-record',
        title: 'Recorded prerequisite contribution',
        summary: 'Made the exact contribution available for later readiness evaluation.',
      },
    ],
  },
];

const evidence = [
  {
    id: 'diff',
    kind: 'diff',
    activityKey: 'implementer-claim',
    activityTitle: 'Implementer claim',
    label: 'src/integration/apply.ts',
    detail: '+24 −3',
    title: 'Accepted file change',
    summary:
      'The bounded integration path now records settlement after the accepted candidate is applied.',
    metadata: [
      { label: 'Revision', value: 'revision-integration-review' },
      { label: 'Compared with', value: 'accepted parent' },
    ],
    lines: [
      { type: 'context', value: 'export async function applyAcceptedCandidate(candidate) {' },
      { type: 'add', value: '  const integration = await repository.apply(candidate);' },
      { type: 'add', value: '  await repository.recordSettlement(integration.workUnitId);' },
      { type: 'add', value: '  return integration;' },
      { type: 'context', value: '}' },
    ],
    available: true,
  },
  {
    id: 'build',
    kind: 'test',
    activityKey: 'implementer-claim',
    activityTitle: 'Implementer claim',
    label: 'Focused integration test',
    detail: '4 passed · 12.8 sec',
    title: 'Integration settlement validation',
    summary:
      'Checks that an accepted candidate is applied once, the Work Unit settles, and dependent readiness is reevaluated without activating dependent work.',
    metadata: [
      {
        label: 'Command',
        value: 'npm test -- --run src/integration/apply.test.ts',
      },
      { label: 'Environment', value: 'fixture-backed local runner' },
      { label: 'Exit code', value: '0' },
    ],
    cases: [
      'applies the accepted candidate exactly once',
      'records Work Unit settlement',
      'preserves unavailable evidence explicitly',
      'contributes prerequisites without starting dependent work',
    ],
    available: true,
  },
  {
    id: 'manifest',
    kind: 'unavailable',
    activityKey: 'application-evidence',
    activityTitle: 'Application evidence',
    label: 'Integration manifest',
    detail: 'Evidence unavailable',
    title: 'Integration manifest unavailable',
    summary: 'No manifest record was captured for this fixture, so no detail view is available.',
    metadata: [],
    available: false,
  },
] as const;

function formatDateTime(value: string) {
  return new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
    timeZone: 'UTC',
    timeZoneName: 'short',
  }).format(new Date(value));
}

function formatDuration(startedAt: string, completedAt: string) {
  const seconds = Math.max(
    0,
    Math.round((new Date(completedAt).getTime() - new Date(startedAt).getTime()) / 1000),
  );
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;

  if (minutes === 0) return `${remainingSeconds} sec`;
  if (remainingSeconds === 0) return `${minutes} min`;
  return `${minutes} min ${remainingSeconds} sec`;
}

export function IntegrationSettlementReviewHarness() {
  const [selectedKey, setSelectedKey] = useState<ActivityKey>('handler-review');
  const [hoveredKey, setHoveredKey] = useState<ActivityKey | null>(null);
  const [technicalOpen, setTechnicalOpen] = useState(false);
  const [currentWorkOpen, setCurrentWorkOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<'session' | 'evidence'>('session');
  const [selectedEvidence, setSelectedEvidence] = useState<string | null>(null);
  const [hoveredEvidenceId, setHoveredEvidenceId] = useState<string | null>(null);
  const [inspectedTurnId, setInspectedTurnId] = useState<string | null>(null);
  const [expandedSystemRecordId, setExpandedSystemRecordId] = useState<string | null>(null);
  const [expandedSteps, setExpandedSteps] = useState<ReadonlySet<string>>(new Set());
  const turnRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const activeKey = hoveredKey ?? selectedKey;
  const timelineTurns = [...turns].sort(
    (left, right) => new Date(left.startedAt).getTime() - new Date(right.startedAt).getTime(),
  );
  const agentTurns = timelineTurns.filter((turn) => turn.kind === 'agent');
  const primaryStages = stages.filter((stage) => !applicationStageParents[stage.key]);
  const selectedIndex = stages.findIndex((stage) => stage.key === selectedKey);
  const selectedStage = stages[selectedIndex];
  const selectedTurn = turns.find((turn) => turn.id === selectedStage.turnId)!;
  const inspectedTurn = turns.find((turn) => turn.id === inspectedTurnId) ?? null;
  const evidenceRecord = selectedEvidence
    ? (evidence.find((record) => record.id === selectedEvidence) ?? null)
    : null;

  useEffect(() => {
    if (activeTab === 'session' && selectedTurn.kind === 'agent') {
      turnRefs.current[selectedTurn.id]?.focus({ preventScroll: true });
    }
  }, [activeTab, selectedTurn]);

  function inspectTurn(turn: SessionTurn) {
    setActiveTab('session');
    setSelectedKey(turn.activityKey);
    setInspectedTurnId((current) => (current === turn.id ? null : turn.id));
    setExpandedSystemRecordId(null);
    setExpandedSteps(new Set());
  }

  function inspectActivity(key: ActivityKey) {
    const stage = stages.find((candidate) => candidate.key === key);
    const turn = turns.find((candidate) => candidate.id === stage?.turnId);
    if (!turn) return;
    if (turn.kind === 'agent') inspectTurn(turn);
    else {
      setSelectedKey(key);
      setInspectedTurnId(null);
      setExpandedSystemRecordId(turn.id);
    }
  }

  function toggleSystemRecord(record: SessionTurn) {
    setSelectedKey(record.activityKey);
    setExpandedSystemRecordId((current) => (current === record.id ? null : record.id));
  }

  function toggleStep(stepId: string) {
    setExpandedSteps((current) => {
      const next = new Set(current);
      if (next.has(stepId)) next.delete(stepId);
      else next.add(stepId);
      return next;
    });
  }

  function toggleAllSteps() {
    if (!inspectedTurn) return;
    const allExpanded = inspectedTurn.steps.every((step) => expandedSteps.has(step.id));
    setExpandedSteps(allExpanded ? new Set() : new Set(inspectedTurn.steps.map((step) => step.id)));
  }

  function moveSelection(offset: number) {
    const next = Math.min(Math.max(selectedIndex + offset, 0), stages.length - 1);
    inspectActivity(stages[next].key);
  }

  function renderSystemRecords(records: readonly SessionTurn[], label: string) {
    if (records.length === 0) return null;

    return (
      <ul className="integration-activity__system-records" aria-label={label}>
        {records.map((record) => {
          const expanded = expandedSystemRecordId === record.id;
          return (
            <li key={record.id}>
              <button
                type="button"
                aria-expanded={expanded}
                onClick={() => toggleSystemRecord(record)}
              >
                <span aria-hidden="true">
                  {record.kind === 'mcp' ? <Wrench size={14} /> : <Layers3 size={14} />}
                </span>
                <span>
                  <strong>{record.name}</strong>
                  <small>
                    {record.role} · {formatDuration(record.startedAt, record.completedAt)}
                  </small>
                </span>
                {expanded ? (
                  <ChevronDown size={14} aria-hidden="true" />
                ) : (
                  <ChevronRight size={14} aria-hidden="true" />
                )}
              </button>
              {expanded && (
                <div className="integration-activity__system-detail">
                  <time>{formatDateTime(record.completedAt)}</time>
                  <p>{record.fullOutput}</p>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    );
  }

  return (
    <main className="integration-review">
      <header className="integration-review__actions" aria-label="Work Unit navigation">
        <button type="button" className="integration-review__back" onClick={() => undefined}>
          <ArrowLeft size={18} aria-hidden="true" />
          Back to Work Slice planning point
        </button>
        <span>Accepted integration review</span>
        <div className="integration-review__current">
          <button
            type="button"
            aria-expanded={currentWorkOpen}
            onClick={() => setCurrentWorkOpen((open) => !open)}
          >
            <span aria-hidden="true" />
            Current work
            <ChevronDown size={16} aria-hidden="true" />
          </button>
          {currentWorkOpen && (
            <div className="integration-review__current-popover">
              <strong>Settled</strong>
              <span>No active work remains.</span>
            </div>
          )}
        </div>
      </header>

      <section className="integration-review__identity" aria-label="Work Unit summary">
        <div className="integration-review__title">
          <span className="integration-review__work-icon" aria-hidden="true">
            <ClipboardCheck size={23} />
          </span>
          <span>
            <small>Work Unit</small>
            <strong>Integrate one accepted Work Unit</strong>
          </span>
        </div>
        <div className="integration-review__purpose">
          <small>Purpose</small>
          <span>Apply the accepted candidate to the authoritative Sprint state.</span>
        </div>
        <div className="integration-review__status">
          <small>Status</small>
          <span>
            <Check size={15} aria-hidden="true" /> Settled
          </span>
        </div>
        <div className="integration-review__technical">
          <button
            type="button"
            aria-expanded={technicalOpen}
            onClick={() => setTechnicalOpen((open) => !open)}
          >
            <Wrench size={16} aria-hidden="true" />
            Technical identity
            <ChevronDown size={16} aria-hidden="true" />
          </button>
          {technicalOpen && (
            <dl>
              <div>
                <dt>Work Unit</dt>
                <dd>unit-integration-review</dd>
              </div>
              <div>
                <dt>Attempt</dt>
                <dd>attempt-1</dd>
              </div>
              <div>
                <dt>Revision</dt>
                <dd>revision-integration-review</dd>
              </div>
            </dl>
          )}
        </div>
      </section>

      <div className="integration-review__workspace">
        <section className="integration-activity" aria-labelledby="integration-activity-heading">
          <header>
            <h1 id="integration-activity-heading">Integration activity</h1>
            <span>{stages.length} events</span>
          </header>
          <ol>
            {primaryStages.map((stage) => {
              const Icon = stage.icon;
              const active = activeKey === stage.key;
              const applicationStages = stages.filter(
                (candidate) => applicationStageParents[candidate.key] === stage.key,
              );
              return (
                <li key={stage.key} className={active ? 'is-related' : undefined}>
                  <button
                    type="button"
                    aria-pressed={selectedKey === stage.key}
                    onMouseEnter={() => setHoveredKey(stage.key)}
                    onMouseLeave={() => setHoveredKey(null)}
                    onFocus={() => setHoveredKey(stage.key)}
                    onBlur={() => setHoveredKey(null)}
                    onClick={() => inspectActivity(stage.key)}
                  >
                    <span className="integration-activity__icon" aria-hidden="true">
                      <Icon size={18} />
                    </span>
                    <span className="integration-activity__copy">
                      <span className="integration-activity__heading">
                        <strong>{stage.title}</strong>
                        <span>
                          <CheckCircle2 size={14} aria-hidden="true" /> {stage.status}
                        </span>
                      </span>
                      <small>
                        {formatDateTime(stage.completedAt)} <i aria-hidden="true">·</i>{' '}
                        {stage.owner} <i aria-hidden="true">·</i>{' '}
                        <Clock3 size={12} aria-hidden="true" />{' '}
                        {formatDuration(stage.startedAt, stage.completedAt)}
                      </small>
                      <span>{stage.summary}</span>
                    </span>
                  </button>
                  {applicationStages.length > 0 && (
                    <ul className="integration-activity__application-chain">
                      {applicationStages.map((applicationStage) => {
                        const ApplicationIcon = applicationStage.icon;
                        const applicationActive = activeKey === applicationStage.key;
                        const systemRecords = timelineTurns.filter(
                          (turn) =>
                            turn.activityKey === applicationStage.key && turn.kind !== 'agent',
                        );
                        return (
                          <li
                            key={applicationStage.key}
                            className={applicationActive ? 'is-related' : undefined}
                          >
                            <button
                              type="button"
                              className="integration-activity__application-stage"
                              aria-pressed={selectedKey === applicationStage.key}
                              onMouseEnter={() => setHoveredKey(applicationStage.key)}
                              onMouseLeave={() => setHoveredKey(null)}
                              onFocus={() => setHoveredKey(applicationStage.key)}
                              onBlur={() => setHoveredKey(null)}
                              onClick={() => inspectActivity(applicationStage.key)}
                            >
                              <span className="integration-activity__icon" aria-hidden="true">
                                <ApplicationIcon size={16} />
                              </span>
                              <span className="integration-activity__copy">
                                <span className="integration-activity__heading">
                                  <strong>{applicationStage.title}</strong>
                                  <span>
                                    <CheckCircle2 size={13} aria-hidden="true" />{' '}
                                    {applicationStage.status}
                                  </span>
                                </span>
                                <small>
                                  {formatDateTime(applicationStage.completedAt)} · Application ·{' '}
                                  <Clock3 size={11} aria-hidden="true" />{' '}
                                  {formatDuration(
                                    applicationStage.startedAt,
                                    applicationStage.completedAt,
                                  )}
                                </small>
                                <span>{applicationStage.summary}</span>
                              </span>
                            </button>
                            {renderSystemRecords(
                              systemRecords,
                              `${applicationStage.title} application records`,
                            )}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </li>
              );
            })}
          </ol>
        </section>

        <section
          className={`integration-session integration-session--${activeTab}`}
          aria-labelledby="integration-session-heading"
        >
          <header>
            <nav aria-label="Work Unit detail" role="tablist">
              <button
                id="integration-session-heading"
                type="button"
                role="tab"
                aria-selected={activeTab === 'session'}
                onClick={() => setActiveTab('session')}
              >
                Session stream
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={activeTab === 'evidence'}
                onClick={() => setActiveTab('evidence')}
              >
                Evidence <span>{evidence.filter((record) => record.available).length}</span>
              </button>
            </nav>
            <span className="integration-session__accepted">
              <CheckCircle2 size={14} aria-hidden="true" /> Accepted
            </span>
          </header>

          {activeTab === 'session' ? (
            <>
              <div
                className="integration-session__turns"
                aria-label="Merged Agent Session passages"
              >
                {agentTurns.map((turn, index) => {
                  const active = activeKey === turn.activityKey;
                  const expanded = inspectedTurnId === turn.id;
                  return (
                    <Fragment key={turn.id}>
                      <button
                        ref={(element) => {
                          turnRefs.current[turn.id] = element;
                        }}
                        type="button"
                        className={active ? 'is-related' : undefined}
                        aria-expanded={expanded}
                        aria-pressed={selectedTurn.id === turn.id}
                        onMouseEnter={() => setHoveredKey(turn.activityKey)}
                        onMouseLeave={() => setHoveredKey(null)}
                        onFocus={() => setHoveredKey(turn.activityKey)}
                        onBlur={() => setHoveredKey(null)}
                        onClick={() => inspectTurn(turn)}
                      >
                        <span className="integration-session__avatar" aria-hidden="true">
                          {turn.role === 'Handler' ? <ShieldCheck size={18} /> : <Bot size={18} />}
                        </span>
                        <span className="integration-session__turn-copy">
                          <span>
                            <strong>{turn.name}</strong>
                            <small>{turn.role}</small>
                            <small className="integration-session__duration">
                              <Clock3 size={12} aria-hidden="true" />{' '}
                              {formatDuration(turn.startedAt, turn.completedAt)}
                            </small>
                          </span>
                          <span className="integration-session__turn-meta">
                            {formatDateTime(turn.completedAt)}
                          </span>
                          <span>{turn.text}</span>
                        </span>
                        <small className="integration-session__reference">
                          <Link2 size={13} aria-hidden="true" /> Ref {index + 1}
                        </small>
                      </button>
                      {expanded && (
                        <article
                          className="integration-session__inspector is-inline"
                          aria-label={`${turn.name} full turn output`}
                        >
                          <div className="integration-session__inline-context">
                            <span>Read-only session turn</span>
                            <small>
                              {formatDateTime(turn.completedAt)} ·{' '}
                              {formatDuration(turn.startedAt, turn.completedAt)} processing
                            </small>
                          </div>
                          <section
                            className="integration-session__output"
                            aria-labelledby={`turn-output-title-${turn.id}`}
                          >
                            <h3 id={`turn-output-title-${turn.id}`}>Full output</h3>
                            <p>{turn.fullOutput}</p>
                          </section>
                          <section
                            className="integration-session__steps"
                            aria-labelledby={`turn-steps-title-${turn.id}`}
                          >
                            <header>
                              <div>
                                <h3 id={`turn-steps-title-${turn.id}`}>Turn steps</h3>
                                <span>{turn.steps.length} recorded steps</span>
                              </div>
                              <button type="button" onClick={toggleAllSteps}>
                                {turn.steps.every((step) => expandedSteps.has(step.id))
                                  ? 'Collapse all'
                                  : 'Expand all'}
                              </button>
                            </header>
                            <ol>
                              {turn.steps.map((step, stepIndex) => {
                                const stepExpanded = expandedSteps.has(step.id);
                                return (
                                  <li key={step.id}>
                                    <button
                                      type="button"
                                      aria-expanded={stepExpanded}
                                      onClick={() => toggleStep(step.id)}
                                    >
                                      <span>{stepIndex + 1}</span>
                                      <strong>{step.title}</strong>
                                      {stepExpanded ? (
                                        <ChevronDown size={16} aria-hidden="true" />
                                      ) : (
                                        <ChevronRight size={16} aria-hidden="true" />
                                      )}
                                    </button>
                                    {stepExpanded && <p>{step.summary}</p>}
                                  </li>
                                );
                              })}
                            </ol>
                          </section>
                        </article>
                      )}
                    </Fragment>
                  );
                })}
              </div>

              <footer>
                <button
                  type="button"
                  disabled={selectedIndex === 0}
                  onClick={() => moveSelection(-1)}
                >
                  <ArrowUp size={16} aria-hidden="true" /> Previous activity
                </button>
                <span>
                  {selectedIndex + 1} of {stages.length} linked activities
                </span>
                <button
                  type="button"
                  disabled={selectedIndex === stages.length - 1}
                  onClick={() => moveSelection(1)}
                >
                  Next activity <ArrowDown size={16} aria-hidden="true" />
                </button>
              </footer>
            </>
          ) : (
            <section className="integration-evidence-tab" aria-labelledby="evidence-tab-title">
              <header>
                <div>
                  <h2 id="evidence-tab-title">Work Unit evidence</h2>
                  <span>Application-owned records for this exact attempt</span>
                </div>
                <p>
                  Select an available record to inspect its captured detail. Unavailable evidence
                  stays visible and cannot be inferred.
                </p>
              </header>
              <div className="integration-evidence-tab__workspace">
                <nav aria-label="Evidence records">
                  {evidence.map((record) => (
                    <button
                      key={record.id}
                      type="button"
                      disabled={!record.available}
                      aria-pressed={selectedEvidence === record.id}
                      onMouseEnter={() => {
                        setHoveredKey(record.activityKey);
                        setHoveredEvidenceId(record.id);
                      }}
                      onMouseLeave={() => {
                        setHoveredKey(null);
                        setHoveredEvidenceId(null);
                      }}
                      onFocus={() => {
                        setHoveredKey(record.activityKey);
                        setHoveredEvidenceId(record.id);
                      }}
                      onBlur={() => {
                        setHoveredKey(null);
                        setHoveredEvidenceId(null);
                      }}
                      onClick={() => setSelectedEvidence(record.id)}
                    >
                      {record.kind === 'diff' ? (
                        <FileCode2 size={17} aria-hidden="true" />
                      ) : (
                        <CheckCircle2 size={17} aria-hidden="true" />
                      )}
                      <span>
                        <strong>{record.label}</strong>
                        <small>{record.detail}</small>
                        {record.available && (
                          <span
                            className={`integration-evidence-tab__link${
                              hoveredEvidenceId === record.id ? ' is-visible' : ''
                            }`}
                          >
                            <Link2 size={12} aria-hidden="true" /> Linked to {record.activityTitle}
                          </span>
                        )}
                      </span>
                      {record.available && <ChevronRight size={16} aria-hidden="true" />}
                    </button>
                  ))}
                </nav>
                {evidenceRecord ? (
                  <article aria-labelledby="evidence-detail-title">
                    <header>
                      <span>{evidenceRecord.kind === 'diff' ? 'File diff' : 'Test execution'}</span>
                      <h3 id="evidence-detail-title">{evidenceRecord.title}</h3>
                      <p>{evidenceRecord.summary}</p>
                    </header>
                    <dl>
                      {evidenceRecord.metadata.map((item) => (
                        <div key={item.label}>
                          <dt>{item.label}</dt>
                          <dd>{item.value}</dd>
                        </div>
                      ))}
                    </dl>
                    {'lines' in evidenceRecord && (
                      <pre className="integration-evidence-tab__diff" aria-label="File comparison">
                        {evidenceRecord.lines.map((line, index) => (
                          <code key={`${line.value}-${index}`} className={`is-${line.type}`}>
                            <span>{index + 18}</span>
                            {line.type === 'add' ? '+' : ' '} {line.value}
                          </code>
                        ))}
                      </pre>
                    )}
                    {'cases' in evidenceRecord && (
                      <section
                        className="integration-evidence-tab__tests"
                        aria-label="Passing tests"
                      >
                        <h4>4 passing tests</h4>
                        <ul>
                          {evidenceRecord.cases.map((testCase) => (
                            <li key={testCase}>
                              <CheckCircle2 size={15} aria-hidden="true" /> {testCase}
                            </li>
                          ))}
                        </ul>
                      </section>
                    )}
                  </article>
                ) : (
                  <div className="integration-evidence-tab__empty" role="status">
                    <FileCheck2 size={28} aria-hidden="true" />
                    <strong>Select evidence to inspect</strong>
                    <p>Choose an available file or test record to open its captured detail.</p>
                  </div>
                )}
              </div>
            </section>
          )}
        </section>
      </div>
    </main>
  );
}
