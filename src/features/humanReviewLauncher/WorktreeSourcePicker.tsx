import { FolderGit2, GitBranch, GitCompare, Search } from 'lucide-react';
import { useMemo, useState } from 'react';
import type { HumanReviewSource } from '../../application/humanReviewLauncher';

export function WorktreeSourcePicker({
  sources,
  selectedSourceRef,
  disabled,
  historyLoading,
  attaching,
  onSelect,
  onAttach,
  onViewHistory,
}: {
  readonly sources: readonly HumanReviewSource[];
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly historyLoading: boolean;
  readonly attaching: boolean;
  readonly onSelect: (sourceRef: string) => void;
  readonly onAttach: (sourceRef: string) => void;
  readonly onViewHistory: (sourceRef: string, trigger: HTMLButtonElement) => void;
}) {
  const [query, setQuery] = useState('');
  const visible = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    return sources
      .filter((source) => !source.detached)
      .filter((source) =>
        normalized ? `${source.branch ?? ''} ${source.label}`.toLocaleLowerCase().includes(normalized) : true,
      )
      .sort((left, right) => Number(right.isMain) - Number(left.isMain) || (left.branch ?? '').localeCompare(right.branch ?? ''));
  }, [query, sources]);
  const selected = sources.find((source) => source.sourceRef === selectedSourceRef);

  return (
    <section className="repository-history" aria-label="Review source">
      <header className="repository-history__header">
        <div>
          <h3>Repository history</h3>
          <p>Find a branch, tag, or archived branch and review how it relates to the comparison branch.</p>
        </div>
        <label className="repository-history__search">
          <Search size={16} aria-hidden="true" />
          <span className="sr-only">Search repository history</span>
          <input
            type="search"
            value={query}
            placeholder="Search branches and tags"
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
      </header>

      <div className="repository-history__graph" aria-label="Branch history">
        {visible.map((source) => {
          const active = source.sourceRef === selectedSourceRef;
          return (
            <button
              key={source.sourceRef}
              type="button"
              data-main={source.isMain}
              className={active ? 'repository-history__line is-selected' : 'repository-history__line'}
              aria-pressed={active}
              disabled={disabled}
              onClick={() => onSelect(source.sourceRef)}
            >
              <span className="repository-history__node" aria-hidden="true" />
              <GitBranch size={17} aria-hidden="true" />
              <span className="repository-history__branch">
                <strong>{source.branch ?? source.label}</strong>
                <small>{source.refKind === 'archive' ? 'Archived branch' : readableKind(source.refKind)}</small>
              </span>
              <span className="repository-history__track" aria-hidden="true">
                <i />
                <i />
                <i />
              </span>
              <span className={source.attached ? 'repository-history__attachment is-attached' : 'repository-history__attachment'}>
                <FolderGit2 size={15} aria-hidden="true" />
                {source.attached ? 'Review worktree ready' : 'Review worktree needed'}
              </span>
              <span className="repository-history__revision">{source.revision}</span>
            </button>
          );
        })}
        {visible.length === 0 && <p className="repository-history__empty">No matching branches or tags.</p>}
      </div>

      <aside className="repository-history__details" aria-live="polite">
        {selected ? (
          <>
            <div className="repository-history__summary">
              <span className="repository-history__summary-icon"><GitCompare size={19} /></span>
              <div>
                <p>{selected.refKind === 'archive' ? 'Archived branch' : readableKind(selected.refKind)}</p>
                <h3>{selected.branch ?? selected.label}</h3>
                <span>
                  {selected.relationship === 'unrelated'
                    ? `No common ancestor with ${selected.comparisonBranch}`
                    : selected.mergedDirectly
                      ? `Included directly in ${selected.comparisonBranch}`
                      : `Not merged directly into ${selected.comparisonBranch}`}
                  {' · '}{selected.ahead} unique {selected.ahead === 1 ? 'commit' : 'commits'}
                  {' · '}{selected.equivalentPatches === 0 ? 'No equivalent patches found' : `${selected.equivalentPatches} equivalent patches found`}
                </span>
              </div>
            </div>
            <div className="repository-history__actions">
              {!selected.attached ? (
                <>
                  <p>A review worktree must be created before this branch can be built.</p>
                  <button type="button" disabled={disabled || attaching} onClick={() => onAttach(selected.sourceRef)}>
                    <FolderGit2 size={16} />
                    {attaching ? 'Creating review worktree…' : 'Create review worktree'}
                  </button>
                </>
              ) : (
                <p className="repository-history__ready"><FolderGit2 size={16} /> Review worktree is ready to build.</p>
              )}
              <button
                type="button"
                className="repository-history__secondary"
                disabled={disabled || historyLoading || selected.relationship === 'unrelated'}
                onClick={(event) => onViewHistory(selected.sourceRef, event.currentTarget)}
              >
                {historyLoading ? 'Loading history…' : `Compare with ${selected.comparisonBranch}`}
              </button>
            </div>
          </>
        ) : (
          <p>Select a branch or tag to inspect it.</p>
        )}
      </aside>
    </section>
  );
}

function readableKind(kind: HumanReviewSource['refKind']) {
  if (kind === 'remote_branch') return 'Remote branch';
  if (kind === 'tag') return 'Tag';
  if (kind === 'detached') return 'Detached revision';
  return 'Branch';
}
