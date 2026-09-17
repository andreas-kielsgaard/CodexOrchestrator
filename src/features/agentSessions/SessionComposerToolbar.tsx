import { FolderGit2, Monitor, Shield } from 'lucide-react';
import {
  localExecutionBinding,
  type SessionExecutionSelectionDto,
} from '../../application/executionTargets/contracts';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';
import { targetWorktreeLabel } from '../../application/executionTargets/presentation';
import './sessionPreparation.css';
export interface SessionComposerToolbarProps {
  profiles: readonly CapabilityProfileDto[];
  selection: SessionExecutionSelectionDto | null;
  deviceId: string | null;
  options: PerMessageRuntimeSelection;
  models: readonly string[];
  reasoningModes: readonly string[];
  defaultModel?: string | null;
  defaultReasoning?: string | null;
  pending: boolean;
  onProfile(id: string): void;
  onDevice(): void;
  onWorktree(): void;
  onOptions(value: PerMessageRuntimeSelection): void;
  onPreview(): void;
}
export function SessionComposerToolbar(props: SessionComposerToolbarProps) {
  const workspace = props.selection?.workspace;
  const worktreeLabel =
    workspace?.kind === 'existing'
      ? workspace.target.branchRef
        ? targetWorktreeLabel(workspace.target.branchRef, workspace.target.path)
        : 'Session folder'
      : workspace?.kind === 'create'
        ? `${workspace.branchRef.replace(/^refs\/heads\//, '')} · new at ${workspace.commit.slice(0, 8)}`
        : 'Select branch';
  return (
    <>
      <div className="session-composer-toolbar__targets">
        <label className="session-composer-toolbar__select">
          <Shield size={16} aria-hidden="true" />
          <select
            aria-label="Capability Profile"
            value={props.selection?.capabilityProfileId ?? ''}
            onChange={(event) => props.onProfile(event.target.value)}
            disabled={props.profiles.length === 0}
          >
            {props.profiles.length === 0 && <option value="">No Capability Profiles</option>}
            {props.profiles
              .filter(
                (profile) =>
                  !props.deviceId ||
                  (profile.execution ?? localExecutionBinding).deviceId === props.deviceId,
              )
              .map((profile) => (
                <option key={profile.capabilityProfileId} value={profile.capabilityProfileId}>
                  {profile.name}
                </option>
              ))}
          </select>
        </label>
        <button
          type="button"
          className="session-composer-toolbar__device"
          aria-label={`Destination device: ${props.selection?.execution.deviceName ?? 'Choose device'}`}
          onClick={props.onDevice}
          title="Compare this conversation’s worktree on another device"
        >
          <Monitor size={16} aria-hidden="true" />
          <span>{props.selection?.execution.deviceName ?? 'Destination device'}</span>
        </button>
        <button
          className="session-composer-toolbar__worktree"
          aria-label={
            workspace?.kind === 'existing'
              ? `${worktreeLabel} · ${props.selection?.execution.deviceName}`
              : workspace?.kind === 'create'
                ? worktreeLabel
                : 'Target worktree'
          }
          type="button"
          onClick={props.onWorktree}
          title={
            workspace?.kind === 'existing'
              ? `${workspace.target.repositoryId} · ${workspace.target.path} · ${workspace.target.worktreeId}`
              : worktreeLabel
          }
        >
          <FolderGit2 size={16} aria-hidden="true" />
          <span>{worktreeLabel}</span>
        </button>
        {props.pending && (
          <button
            type="button"
            className="session-preparation-marker"
            aria-label="Preparation required on Send. View steps"
            title="Preparation required on Send. View steps"
            onClick={props.onPreview}
          >
            <span />
          </button>
        )}
      </div>
      <div
        className="session-composer-toolbar__runtime"
        role="group"
        aria-label="Model and reasoning"
      >
        <select
          aria-label="Model"
          value={props.options.model ?? ''}
          onChange={(event) =>
            props.onOptions({ ...props.options, model: event.target.value || null })
          }
        >
          <option value="">{props.defaultModel ?? 'Model default'}</option>
          {props.options.model && !props.models.includes(props.options.model) && (
            <option value={props.options.model} disabled>
              {props.options.model} · unavailable
            </option>
          )}
          {props.models.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </select>
        <select
          aria-label="Reasoning"
          value={props.options.reasoningMode ?? ''}
          onChange={(event) =>
            props.onOptions({ ...props.options, reasoningMode: event.target.value || null })
          }
        >
          <option value="">{props.defaultReasoning ?? 'Reasoning default'}</option>
          {props.options.reasoningMode &&
            !props.reasoningModes.includes(props.options.reasoningMode) && (
              <option value={props.options.reasoningMode} disabled>
                {props.options.reasoningMode} · unavailable
              </option>
            )}
          {props.reasoningModes.map((mode) => (
            <option key={mode} value={mode}>
              {mode}
            </option>
          ))}
        </select>
      </div>
    </>
  );
}
