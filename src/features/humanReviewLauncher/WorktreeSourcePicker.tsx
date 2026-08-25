import { FolderGit2, GitBranch, History } from 'lucide-react';
import type { HumanReviewSource } from '../../application/humanReviewLauncher';

export function WorktreeSourcePicker({
  sources,
  selectedSourceRef,
  disabled,
  historyLoading,
  onSelect,
  onExplore,
  onViewHistory,
}: {
  readonly sources: readonly HumanReviewSource[];
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly historyLoading: boolean;
  readonly onSelect: (sourceRef: string) => void;
  readonly onExplore: () => void;
  readonly onViewHistory: (sourceRef: string, trigger: HTMLButtonElement) => void;
}) {
  const attached = sources
    .filter((source) => source.attached)
    .sort(
      (left, right) =>
        Number(right.isCurrent) - Number(left.isCurrent) ||
        Number(right.isMain) - Number(left.isMain) ||
        branchName(left).localeCompare(branchName(right)),
    );
  const selected = attached.find((source) => source.sourceRef === selectedSourceRef);

  return (
    <section className="attached-worktrees" aria-label="Review source">
      <div className="attached-worktrees__browser">
        <header>
          <div>
            <h3>Attached worktrees</h3>
            <p>Choose a folder that already represents the branch or revision you want to review.</p>
          </div>
          <button type="button" className="attached-worktrees__explore" onClick={onExplore} disabled={disabled}>
            <History size={16} />
            Explore repository history
          </button>
        </header>
        <div className="attached-worktrees__list">
          {attached.map((source) => {
            const active = source.sourceRef === selectedSourceRef;
            return (
              <button
                key={source.sourceRef}
                type="button"
                className={active ? 'is-selected' : undefined}
                aria-pressed={active}
                disabled={disabled}
                onClick={() => onSelect(source.sourceRef)}
              >
                <FolderGit2 size={18} />
                <span>
                  <strong>{worktreeLabel(source)}</strong>
                  <small><GitBranch size={13} /> {branchName(source)}</small>
                </span>
                <code>{source.revision}</code>
                {source.isCurrent && <em>Current</em>}
              </button>
            );
          })}
          {attached.length === 0 && <p>No attached worktrees are available.</p>}
        </div>
      </div>

      <aside className="attached-worktrees__details" aria-live="polite">
        {selected ? (
          <>
            <p>{selected.isMain ? 'Main checkout' : 'Attached worktree'}</p>
            <h3>{worktreeLabel(selected)}</h3>
            <dl>
              <div><dt>Branch head</dt><dd>{branchName(selected)}</dd></div>
              <div><dt>Revision</dt><dd>{selected.revision}</dd></div>
              <div><dt>Review status</dt><dd>{selected.compatibility === 'compatible' ? 'Ready' : 'Requires update'}</dd></div>
            </dl>
            <button
              type="button"
              disabled={disabled || historyLoading || selected.detached || selected.relationship === 'unrelated'}
              onClick={(event) => onViewHistory(selected.sourceRef, event.currentTarget)}
            >
              {historyLoading ? 'Loading history…' : 'View commit history'}
            </button>
          </>
        ) : (
          <p>Select an attached worktree.</p>
        )}
      </aside>
    </section>
  );
}

function branchName(source: HumanReviewSource) {
  return source.branch ?? `Detached ${source.revision.slice(0, 8)}`;
}

function worktreeLabel(source: HumanReviewSource) {
  if (source.isMain) return 'Main checkout';
  if (source.isCurrent) return 'Launcher worktree';
  const separator = source.label.lastIndexOf(' - ');
  return separator >= 0 ? source.label.slice(separator + 3) : source.label;
}
