import { GitBranch, GitCommitHorizontal } from 'lucide-react';
import { useState } from 'react';
import type { HumanReviewSource } from '../../application/humanReviewLauncher';

export function WorktreeSourcePicker({
  sources,
  selectedSourceRef,
  disabled,
  historyLoading,
  onSelect,
  onViewHistory,
}: {
  readonly sources: readonly HumanReviewSource[];
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly historyLoading: boolean;
  readonly onSelect: (sourceRef: string) => void;
  readonly onViewHistory: (sourceRef: string, trigger: HTMLButtonElement) => void;
}) {
  const [showDetached, setShowDetached] = useState(false);
  const selected = sources.find((source) => source.sourceRef === selectedSourceRef);
  const named = sources.filter(
    (source) => (!source.detached || source.isMain) && source.relationship === 'related',
  );
  const unrelated = sources.filter(
    (source) => !source.detached && source.relationship === 'unrelated',
  );
  const detached = sources.filter((source) => source.detached && !source.isMain);
  const main = named.find((source) => source.isMain);
  const children = new Map<string, HumanReviewSource[]>();
  named.forEach((source) => {
    if (source.isMain) return;
    const parent =
      source.parentSourceRef &&
      named.some((candidate) => candidate.sourceRef === source.parentSourceRef)
        ? source.parentSourceRef
        : main?.sourceRef;
    if (!parent) return;
    children.set(parent, [...(children.get(parent) ?? []), source]);
  });
  children.forEach((items) =>
    items.sort((left, right) => branchName(left).localeCompare(branchName(right))),
  );

  return (
    <section className="worktree-source-picker" aria-label="Review source">
      <div className="worktree-source-picker__browser">
        <header>
          <div>
            <h3>Branch map</h3>
            <p>Registered worktrees, rooted at the main checkout.</p>
          </div>
          <label className="worktree-source-picker__toggle">
            <input
              type="checkbox"
              role="switch"
              checked={showDetached}
              onChange={(event) => setShowDetached(event.target.checked)}
            />
            Show detached
          </label>
        </header>
        <div className="worktree-source-picker__tree">
          {main ? (
            <BranchNode
              source={main}
              children={children}
              selectedSourceRef={selectedSourceRef}
              disabled={disabled}
              depth={0}
              onSelect={onSelect}
            />
          ) : (
            named.map((source) => (
              <BranchNode
                key={source.sourceRef}
                source={source}
                children={children}
                selectedSourceRef={selectedSourceRef}
                disabled={disabled}
                depth={0}
                onSelect={onSelect}
              />
            ))
          )}
          {showDetached && detached.length > 0 && (
            <div className="worktree-source-picker__detached">
              <p>Detached worktrees</p>
              {detached.map((source) => (
                <button
                  key={source.sourceRef}
                  type="button"
                  className={source.sourceRef === selectedSourceRef ? 'is-selected' : undefined}
                  aria-pressed={source.sourceRef === selectedSourceRef}
                  disabled={disabled}
                  onClick={() => onSelect(source.sourceRef)}
                >
                  <GitCommitHorizontal size={16} />
                  <span>
                    <strong>{branchName(source)}</strong>
                    <small>{source.revision}</small>
                  </span>
                </button>
              ))}
            </div>
          )}
          {unrelated.length > 0 && (
            <div className="worktree-source-picker__detached">
              <p>Other Git histories</p>
              {unrelated.map((source) => (
                <button
                  key={source.sourceRef}
                  type="button"
                  className={source.sourceRef === selectedSourceRef ? 'is-selected' : undefined}
                  aria-pressed={source.sourceRef === selectedSourceRef}
                  disabled={disabled}
                  onClick={() => onSelect(source.sourceRef)}
                >
                  <GitBranch size={16} />
                  <span>
                    <strong>{branchName(source)}</strong>
                    <small>No common ancestor with main</small>
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      <aside className="worktree-source-picker__details" aria-live="polite">
        {selected ? (
          <>
            <div className="worktree-source-picker__details-title">
              <GitBranch size={18} />
              <div>
                <p>
                  {selected.detached
                    ? 'Detached worktree'
                    : selected.isMain
                      ? 'Main checkout'
                      : 'Branch'}
                </p>
                <h3>{branchName(selected)}</h3>
              </div>
            </div>
            <dl>
              <div>
                <dt>Compatibility</dt>
                <dd>
                  {selected.compatibility === 'compatible' ? 'Ready for review' : 'Requires update'}
                </dd>
              </div>
              <div>
                <dt>Since main</dt>
                <dd>
                  {selected.relationship === 'unrelated' ? (
                    'No common ancestor'
                  ) : (
                    <>
                      {selected.ahead} {selected.ahead === 1 ? 'commit' : 'commits'} ahead
                      {selected.behind > 0 ? `, ${selected.behind} behind` : ''}
                    </>
                  )}
                </dd>
              </div>
              <div>
                <dt>Fork revision</dt>
                <dd>{selected.forkRevision}</dd>
              </div>
              <div>
                <dt>Current revision</dt>
                <dd>{selected.revision}</dd>
              </div>
            </dl>
            {selected.lineageAmbiguous && (
              <p className="worktree-source-picker__notice">
                Several registered branch tips are equally close, so this branch is shown directly
                under main.
              </p>
            )}
            <button
              type="button"
              className="worktree-source-picker__history-button"
              disabled={
                disabled ||
                selected.detached ||
                selected.isMain ||
                selected.relationship === 'unrelated'
              }
              onClick={(event) => onViewHistory(selected.sourceRef, event.currentTarget)}
            >
              {historyLoading ? 'Loading history…' : 'View commit history'}
            </button>
          </>
        ) : (
          <p>Select a registered worktree.</p>
        )}
      </aside>
    </section>
  );
}

function BranchNode({
  source,
  children,
  selectedSourceRef,
  disabled,
  depth,
  onSelect,
}: {
  readonly source: HumanReviewSource;
  readonly children: ReadonlyMap<string, readonly HumanReviewSource[]>;
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly depth: number;
  readonly onSelect: (sourceRef: string) => void;
}) {
  const descendants = children.get(source.sourceRef) ?? [];
  return (
    <div className="worktree-source-picker__branch" data-depth={depth}>
      <button
        type="button"
        className={source.sourceRef === selectedSourceRef ? 'is-selected' : undefined}
        aria-pressed={source.sourceRef === selectedSourceRef}
        disabled={disabled}
        onClick={() => onSelect(source.sourceRef)}
      >
        <GitBranch size={16} />
        <span>
          <strong>{branchName(source)}</strong>
          <small>
            {source.revision}
            {source.isCurrent ? ' · launcher source' : ''}
          </small>
        </span>
        {source.isMain && <em>main</em>}
      </button>
      {descendants.length > 0 && (
        <div className="worktree-source-picker__children">
          {descendants.map((child) => (
            <BranchNode
              key={child.sourceRef}
              source={child}
              children={children}
              selectedSourceRef={selectedSourceRef}
              disabled={disabled}
              depth={depth + 1}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function branchName(source: HumanReviewSource) {
  if (source.isMain && !source.branch) return 'Main checkout';
  return source.branch ?? `Detached ${source.revision.slice(0, 8)}`;
}
