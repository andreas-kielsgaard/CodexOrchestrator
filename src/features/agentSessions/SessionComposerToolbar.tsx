import { FolderGit2, Monitor, Shield } from 'lucide-react';
import {
  localExecutionBinding,
  type SessionExecutionSelectionDto,
} from '../../application/executionTargets/contracts';
import type { CapabilityProfileDto } from '../../application/executionConfiguration';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';
import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import { effectiveSessionOptions, selectionForModel } from './effectiveSessionOptions';
import { targetWorktreeLabel } from '../../application/executionTargets/presentation';
import './sessionPreparation.css';
export interface SessionComposerToolbarProps {
  profiles: readonly CapabilityProfileDto[];
  selection: SessionExecutionSelectionDto | null;
  deviceId: string | null;
  options: PerMessageRuntimeSelection;
  capabilities?: AgentSessionQuickFeatures;
  workspaceLabel?: string;
  pending: boolean;
  onProfile(id: string): void;
  onDevice(): void;
  onWorktree(): void;
  onOptions(value: PerMessageRuntimeSelection): void;
  onPreview(): void;
}
export function SessionComposerToolbar(props: SessionComposerToolbarProps) {
  const workspace = props.selection?.workspace;
  const effective = effectiveSessionOptions(props.capabilities, props.options);
  const worktreeLabel =
    workspace?.kind === 'existing'
      ? workspace.target.branchRef
        ? targetWorktreeLabel(workspace.target.branchRef, workspace.target.path)
        : 'Session folder'
      : workspace?.kind === 'create'
        ? `${workspace.branchRef.replace(/^refs\/heads\//, '')} · new at ${workspace.commit.slice(0, 8)}`
        : (props.workspaceLabel ?? 'Empty workspace');
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
          value={effective.model?.id ?? ''}
          onChange={(event) => {
            if (props.capabilities && event.target.value)
              props.onOptions(
                selectionForModel(props.capabilities, props.options, event.target.value),
              );
          }}
          disabled={!effective.model}
        >
          {!effective.model && <option value="">Loading models…</option>}
          {props.capabilities?.models.map((model) => (
            <option key={model.id} value={model.id}>
              {model.label}
            </option>
          ))}
        </select>
        <select
          aria-label="Reasoning"
          value={effective.reasoningMode ?? ''}
          onChange={(event) =>
            props.onOptions({ ...props.options, reasoningMode: event.target.value || null })
          }
          disabled={!effective.model?.reasoningModes.length}
        >
          {!effective.reasoningMode && (
            <option value="">
              {!effective.model
                ? 'Loading reasoning…'
                : effective.model.reasoningModes.length
                  ? 'Default'
                  : 'No reasoning levels'}
            </option>
          )}
          {effective.model?.reasoningModes.map((mode) => (
            <option key={mode.id} value={mode.id}>
              {mode.id}
            </option>
          ))}
        </select>
      </div>
    </>
  );
}
