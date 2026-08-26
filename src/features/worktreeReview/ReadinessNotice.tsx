import type { CapabilityAvailability, ReviewRepository } from '../../application/worktreeReview';

export function ReadinessNotice({ repository }: { readonly repository: ReviewRepository }) {
  if (repository.readiness.state === 'ready') return null;
  const capabilities: readonly (readonly [string, CapabilityAvailability])[] = [
    ['Browse', repository.readiness.browse],
    ['Create worktree', repository.readiness.createWorktree],
    ['Build', repository.readiness.build],
    ['Artifact storage', repository.readiness.artifactStorage],
  ];
  const unavailable = capabilities.filter(
    (item): item is readonly [string, { state: 'unavailable'; reason: string }] =>
      item[1].state === 'unavailable',
  );

  return (
    <section
      className={`worktree-review__notice worktree-review__notice--${repository.readiness.state}`}
      aria-label="Repository readiness"
    >
      <strong>
        {repository.readiness.state === 'unavailable'
          ? 'Worktree Review is unavailable for this repository'
          : 'Some Worktree Review actions are unavailable'}
      </strong>
      <ul>
        {unavailable.map(([label, value]) => (
          <li key={label}>
            {label}: {value.reason}
          </li>
        ))}
      </ul>
    </section>
  );
}
