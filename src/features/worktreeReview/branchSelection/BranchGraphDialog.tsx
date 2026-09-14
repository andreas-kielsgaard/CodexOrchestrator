import { useState } from 'react';
import type {
  GraphConnection,
  ReviewBranch,
  ReviewTarget,
  WorktreeReviewClient,
} from '../../../application/worktreeReview';
import { ReviewDialog } from '../ReviewDialog';
import { BranchGraphBrowser } from '../../branches/BranchGraphBrowser';
import { CommitRangeDialog } from './CommitRangeDialog';
import './branchSelection.css';
export function BranchGraphDialog({
  client,
  repositoryId,
  selectedTarget,
  onClose,
  onSelect,
}: {
  readonly client: WorktreeReviewClient;
  readonly repositoryId: string;
  readonly selectedTarget: ReviewTarget | null;
  readonly onClose: () => void;
  readonly onSelect: (target: ReviewTarget) => void;
}) {
  const [range, setRange] = useState<{ segment: GraphConnection; source: ReviewBranch }>();
  return (
    <ReviewDialog labelledBy="branch-graph-title" onClose={onClose} className="branch-graph-dialog">
      <header className="branch-graph__header">
        <div>
          <p className="worktree-review__step">Repository history</p>
          <h2 id="branch-graph-title">Select branch</h2>
          <p>
            Current branches and their shared history. Hover to trace a branch; open a count to
            choose a commit.
          </p>
        </div>
        <button type="button" className="worktree-review__secondary" onClick={onClose}>
          Close
        </button>
      </header>
      <BranchGraphBrowser
        source={client}
        repositoryId={repositoryId}
        selectedTarget={selectedTarget}
        onSelect={onSelect}
        onRange={(segment, source) => setRange({ segment, source })}
      />
      {range && (
        <CommitRangeDialog
          client={client}
          query={{ target: range.source.target, scope: range.segment.scope }}
          source={range.source}
          connection={range.segment}
          onClose={() => setRange(undefined)}
          onSelect={onSelect}
        />
      )}
    </ReviewDialog>
  );
}
