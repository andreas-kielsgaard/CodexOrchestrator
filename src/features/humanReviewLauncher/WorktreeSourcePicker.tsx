import { FolderGit2, GitBranch, GitCommitHorizontal } from 'lucide-react';
import { useState } from 'react';
import type { HumanReviewSource } from '../../application/humanReviewLauncher';

export function WorktreeSourcePicker({
  sources,
  selectedSourceRef,
  disabled,
  historyLoading,
  showDetached: controlledShowDetached,
  onShowDetachedChange,
  onSelect,
  onViewHistory,
}: {
  readonly sources: readonly HumanReviewSource[];
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly historyLoading: boolean;
  readonly showDetached?: boolean;
  readonly onShowDetachedChange?: (showDetached: boolean) => void;
  readonly onSelect: (sourceRef: string) => void;
  readonly onViewHistory: (sourceRef: string, trigger: HTMLButtonElement) => void;
}) {
  const [localShowDetached, setLocalShowDetached] = useState(false);
  const showDetached = controlledShowDetached ?? localShowDetached;
  const setShowDetached = (next: boolean) => {
    setLocalShowDetached(next);
    onShowDetachedChange?.(next);
  };
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
            <p>Attached worktrees, nested under the branch heads they currently represent.</p>
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
              <p>Detached HEADs</p>
              {detached.map((source) => (
                <div key={source.sourceRef} className="worktree-source-picker__detached-head">
                  <div className="worktree-source-picker__branch-head">
                    <GitCommitHorizontal size={16} />
                    <span>
                      <strong>Detached HEAD</strong>
                      <small>{source.revision}</small>
                    </span>
                  </div>
                  <div className="worktree-source-picker__attachments">
                    <WorktreeAttachment
                      source={source}
                      selectedSourceRef={selectedSourceRef}
                      disabled={disabled}
                      onSelect={onSelect}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
          {unrelated.length > 0 && (
            <div className="worktree-source-picker__detached">
              <p>Other Git histories</p>
              {unrelated.map((source) => (
                <div key={source.sourceRef} className="worktree-source-picker__branch">
                  <div className="worktree-source-picker__branch-head">
                    <GitBranch size={16} />
                    <span>
                      <strong>{branchName(source)}</strong>
                      <small>No common ancestor with main</small>
                    </span>
                  </div>
                  <div className="worktree-source-picker__attachments">
                    <WorktreeAttachment
                      source={source}
                      selectedSourceRef={selectedSourceRef}
                      disabled={disabled}
                      onSelect={onSelect}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <aside className="worktree-source-picker__details" aria-live="polite">
        {selected ? (
          <>
            <div className="worktree-source-picker__details-title">
              <FolderGit2 size={18} />
              <div>
                <p>
                  {selected.detached
                    ? 'Detached worktree'
                    : selected.isMain
                      ? 'Main checkout'
                      : 'Attached worktree'}
                </p>
                <h3>{worktreeLabel(selected)}</h3>
              </div>
            </div>
            <dl>
              <div>
                <dt>Branch head</dt>
                <dd>{branchName(selected)}</dd>
              </div>
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
                <dt>Checked out revision</dt>
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
      <div className="worktree-source-picker__branch-head">
        <GitBranch size={16} />
        <span>
          <strong>{branchName(source)}</strong>
          <small>{source.revision}</small>
        </span>
        {source.isMain && <em>main</em>}
      </div>
      <div className="worktree-source-picker__attachments">
        <WorktreeAttachment
          source={source}
          selectedSourceRef={selectedSourceRef}
          disabled={disabled}
          onSelect={onSelect}
        />
      </div>
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

function WorktreeAttachment({
  source,
  selectedSourceRef,
  disabled,
  onSelect,
}: {
  readonly source: HumanReviewSource;
  readonly selectedSourceRef: string;
  readonly disabled: boolean;
  readonly onSelect: (sourceRef: string) => void;
}) {
  const label = worktreeLabel(source);
  return (
    <button
      type="button"
      className={source.sourceRef === selectedSourceRef ? 'is-selected' : undefined}
      aria-label={`${label}, attached to ${branchName(source)}, at ${source.revision}`}
      aria-pressed={source.sourceRef === selectedSourceRef}
      disabled={disabled}
      onClick={() => onSelect(source.sourceRef)}
    >
      <FolderGit2 size={16} />
      <span>
        <strong>{label}</strong>
        <small>
          {source.isCurrent ? 'Launcher source · build from this folder' : 'Build from this folder'}
        </small>
      </span>
    </button>
  );
}

function branchName(source: HumanReviewSource) {
  if (source.isMain && !source.branch) return 'Main checkout';
  return source.branch ?? `Detached ${source.revision.slice(0, 8)}`;
}

function worktreeLabel(source: HumanReviewSource) {
  if (source.isMain) return 'Main checkout';
  if (source.isCurrent) return 'Launcher worktree';
  const separator = source.label.lastIndexOf(' - ');
  return separator >= 0 ? source.label.slice(separator + 3) : 'Attached worktree';
}
