import type { AttentionState, ExecutionState, Task } from '../../../domain/model';

export type AttentionOptionValue = AttentionState;
export type ExecutionOptionValue = ExecutionState;
export type PriorityOptionValue = Task['priority'];

export interface SelectOption<TValue extends string> {
  value: TValue;
  label: string;
}

export const attentionOptions: Array<SelectOption<AttentionOptionValue>> = [
  { value: 'needs_action_now', label: 'Needs action' },
  { value: 'needs_review', label: 'Needs review' },
  { value: 'waiting_on_agent', label: 'Waiting on agent' },
  { value: 'waiting_on_external', label: 'Waiting external' },
  { value: 'consider_later', label: 'Later' },
  { value: 'snoozed', label: 'Snoozed' },
  { value: 'reference_only', label: 'Reference' },
];

export const executionOptions: Array<SelectOption<ExecutionOptionValue>> = [
  { value: 'draft', label: 'Draft' },
  { value: 'queued', label: 'Queued' },
  { value: 'running', label: 'Running' },
  { value: 'blocked', label: 'Blocked' },
  { value: 'completed', label: 'Completed' },
  { value: 'failed', label: 'Failed' },
];

export const priorityOptions: Array<SelectOption<PriorityOptionValue>> = [
  { value: 'low', label: 'Low' },
  { value: 'normal', label: 'Normal' },
  { value: 'high', label: 'High' },
];
