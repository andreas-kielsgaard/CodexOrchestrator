import {
  isSessionPreparing,
  type SessionPreparationDto,
} from '../../application/agentSessions/preparation';
import type { SessionExecutionSelectionDto } from '../../application/executionTargets/contracts';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';
export function samePreparedConfiguration(
  selection: SessionExecutionSelectionDto | null | undefined,
  options: PerMessageRuntimeSelection | undefined,
  preparation: SessionPreparationDto | null,
  acceptedOptions: string | null,
): boolean {
  if (!preparation || preparation.phase !== 'ready' || isSessionPreparing(preparation))
    return false;
  if (JSON.stringify([selection, options]) === acceptedOptions) return true;
  const accepted = preparation.selection;
  if (
    !selection ||
    !accepted ||
    !options ||
    selection.capabilityProfileId !== accepted.capabilityProfileId ||
    selection.capabilityProfileRevision !== accepted.capabilityProfileRevision ||
    JSON.stringify(selection.execution) !== JSON.stringify(accepted.execution)
  )
    return false;
  const target = preparation.resolvedTarget;
  const workspaceMatches =
    target && selection.workspace.kind === 'existing'
      ? selection.workspace.target.worktreeId === target.worktreeId &&
        selection.workspace.target.repositoryId === target.repositoryId
      : JSON.stringify(selection.workspace) === JSON.stringify(accepted.workspace);
  const defaults = preparation.currentResolution?.sessionProfile.pinnedDefaults;
  const resolved = preparation.resolution?.selections;
  return Boolean(
    workspaceMatches &&
    resolved &&
    (options.model ?? defaults?.model ?? null) === resolved.model &&
    (options.reasoningMode ?? defaults?.reasoningMode ?? null) === resolved.reasoningMode,
  );
}
