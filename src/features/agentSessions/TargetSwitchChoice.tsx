import { targetSwitchRequest } from './targetSwitchRequest';
import type {
  SessionExecutionSelectionDto,
  SessionExecutionTargetDto,
} from '../../application/executionTargets/contracts';
import type {
  MigrationStrategyDto,
  TargetSwitchIntentDto,
} from '../../application/agentSessions/sessionQueue';

export function TargetSwitchChoice({
  destination,
  source,
  intent,
  strategy,
  isSister = false,
  onIntent,
  onStrategy,
}: {
  destination: SessionExecutionSelectionDto | null;
  source: SessionExecutionTargetDto | null;
  intent: TargetSwitchIntentDto;
  strategy: MigrationStrategyDto | '';
  isSister?: boolean;
  onIntent(value: TargetSwitchIntentDto): void;
  onStrategy(value: MigrationStrategyDto | ''): void;
}) {
  const canContinue =
    destination &&
    targetSwitchRequest(
      destination,
      source,
      'continue_work',
      destination.workspace.kind === 'create' ? 'create_linked' : 'link_existing',
    );
  return (
    <fieldset className="target-switch-choice">
      <legend>What should this switch do?</legend>
      <label>
        <input
          type="radio"
          name="target-switch-intent"
          checked={intent === 'change_conversation_target'}
          onChange={() => onIntent('change_conversation_target')}
        />
        Change the conversation target
      </label>
      <label>
        <input
          type="radio"
          name="target-switch-intent"
          checked={intent === 'continue_work'}
          disabled={!canContinue}
          onChange={() => onIntent('continue_work')}
        />
        Continue this work at the destination
      </label>
      {intent === 'continue_work' && (
        <label>
          Migration resolution
          <select
            aria-label="Migration resolution"
            value={strategy}
            onChange={(event) => onStrategy(event.target.value as MigrationStrategyDto | '')}
          >
            <option value="">Choose how to migrate…</option>
            {destination?.workspace.kind === 'create' ? (
              <option value="create_linked">Create and link a new worktree</option>
            ) : (
              <>
                {isSister && (
                  <option value="existing_sister">Use the existing sister worktree</option>
                )}
                <option value="link_existing">Link this worktree and migrate into it</option>
              </>
            )}
          </select>
        </label>
      )}
    </fieldset>
  );
}
