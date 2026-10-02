import type { ReactNode } from 'react';
import { ProcessingDisclosure } from './ProcessingDisclosure';
import { SkillReferenceText } from './SkillReferences';
import type { VisibleInvocation } from './AgentSessionTranscript';
export function InvocationActivityTimeline({
  invocation,
  expandedProcessing,
  onToggleProcessing,
  safeActivityDetails,
  processingHeading,
}: {
  readonly invocation: VisibleInvocation;
  readonly expandedProcessing: ReadonlySet<string>;
  readonly onToggleProcessing: (id: string) => void;
  readonly safeActivityDetails: boolean;
  readonly processingHeading?: string;
}) {
  const steering = (invocation.interactions ?? [])
    .filter(
      (interaction) =>
        interaction.kind === 'steering' &&
        interaction.state === 'accepted' &&
        interaction.content.text?.trim(),
    )
    .sort((left, right) => left.sequence - right.sequence || left.id.localeCompare(right.id));
  if (!steering.length)
    return (
      <ProcessingDisclosure
        invocationId={invocation.id}
        startedAt={invocation.startedAt}
        completedAt={invocation.completedAt}
        usage={invocation.usage}
        activity={invocation.processing}
        running={invocation.isActive && !invocation.preparing}
        expanded={expandedProcessing.has(invocation.id)}
        onToggle={() => onToggleProcessing(invocation.id)}
        safeOnly={safeActivityDetails}
        heading={processingHeading}
      />
    );

  const activity = [...invocation.processing].sort(
    (left, right) => left.sequence - right.sequence || left.id.localeCompare(right.id),
  );
  const content: ReactNode[] = [];
  let activityIndex = 0;
  const addActivity = (until: number | null, segment: number) => {
    const start = activityIndex;
    while (
      activityIndex < activity.length &&
      (until === null || activity[activityIndex].sequence < until)
    )
      activityIndex += 1;
    const items = activity.slice(start, activityIndex);
    const isLast = until === null;
    const disclosureId = `${invocation.id}:activity:${segment}`;
    const hasOngoingTool = items.some(
      (item) =>
        item.kind === 'tool' &&
        item.safeDetail?.kind === 'tool' &&
        item.safeDetail.phase === 'started' &&
        !item.completedAt,
    );
    const boundaryAt = (index: number) =>
      steering[index]?.recordedAt ??
      activity.find((item) => item.sequence > (steering[index]?.sequence ?? Infinity))
        ?.recordedAt ??
      invocation.completedAt;
    if (!items.length && !(isLast && invocation.isActive && !invocation.preparing)) return;
    content.push(
      <ProcessingDisclosure
        key={disclosureId}
        invocationId={disclosureId}
        startedAt={segment === 0 ? invocation.startedAt : boundaryAt(segment - 1)}
        completedAt={isLast ? invocation.completedAt : boundaryAt(segment)}
        usage={invocation.usage}
        activity={items}
        running={(isLast || hasOngoingTool) && invocation.isActive && !invocation.preparing}
        expanded={expandedProcessing.has(disclosureId)}
        onToggle={() => onToggleProcessing(disclosureId)}
        safeOnly={safeActivityDetails}
        heading={processingHeading}
      />,
    );
  };

  steering.forEach((interaction, index) => {
    addActivity(interaction.sequence, index);
    content.push(
      <article className="transcript-message user-message" key={interaction.id}>
        <header>
          <span>You</span>
        </header>
        <p>
          <SkillReferenceText text={interaction.content.text!} />
        </p>
      </article>,
    );
  });
  addActivity(null, steering.length);
  return content;
}
