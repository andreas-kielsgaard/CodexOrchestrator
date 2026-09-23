import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';
import type { ComposerQuickAction } from './composerQuickMenuTypes';
import { effectiveSessionOptions, selectionForModel } from './effectiveSessionOptions';

export interface ComposerQuickFeatures {
  readonly contextKey: string;
  readonly catalogue?: AgentSessionQuickFeatures;
  readonly load: () => Promise<AgentSessionQuickFeatures>;
  readonly refresh?: () => Promise<AgentSessionQuickFeatures>;
  readonly selection: PerMessageRuntimeSelection;
  readonly setSelection: (selection: PerMessageRuntimeSelection) => void;
}

export function sessionQuickActions(
  capabilities: AgentSessionQuickFeatures,
  source: ComposerQuickFeatures,
): readonly ComposerQuickAction[] {
  const { selection, setSelection } = source;
  const effective = effectiveSessionOptions(capabilities, selection);
  const model = effective.model;
  const effectiveModel = model?.id ?? null;
  const effectiveReasoning = effective.reasoningMode;
  const disabledReason = undefined;
  const changeModel = (id: string | null) => {
    if (!id) return { replacement: '', notice: 'No model is available' };
    setSelection(selectionForModel(capabilities, selection, id));
    return { replacement: '', notice: 'Model updated for the next message' };
  };
  const skillActions: ComposerQuickAction[] = capabilities.skills.map((skill) => ({
    id: `skill:${skill.id}`,
    label: skill.name,
    description: skill.description,
    keywords: [skill.name],
    run: () => ({
      replacement: `${skill.invocationText} `,
      notice: `Added ${skill.name} to your draft`,
    }),
  }));
  return [
    {
      id: 'model',
      label: 'Model',
      description: effectiveModel ?? 'Choose a model for your next message',
      disabledReason,
      children: [
        ...capabilities.models.map((item): ComposerQuickAction => ({
          id: item.id,
          label: item.label,
          description: item.description,
          selected: item.id === effectiveModel,
          run: () => changeModel(item.id),
        })),
      ],
    },
    {
      id: 'reasoning',
      label: 'Reasoning',
      description: effectiveReasoning ?? 'Choose a reasoning level for your next message',
      disabledReason:
        disabledReason ?? (!model ? 'Choose a model to see its reasoning levels' : undefined),
      children: [
        ...(model?.reasoningModes ?? []).map((mode): ComposerQuickAction => ({
          id: mode.id,
          label: mode.id,
          description: mode.description,
          selected: mode.id === effectiveReasoning,
          run: () => {
            setSelection({ ...selection, reasoningMode: mode.id });
            return { replacement: '', notice: `Next message reasoning: ${mode.id}` };
          },
        })),
      ],
    },
    {
      id: 'skills',
      label: 'Skills',
      description: 'Add a skill to your message',
      keywords: ['skill'],
      children: skillActions,
    },
    ...skillActions,
  ];
}

export function filterQuickActions(actions: readonly ComposerQuickAction[], query: string) {
  const search = query.trim().toLocaleLowerCase();
  const matches = actions.filter((action) =>
    [action.id, action.label, ...(action.keywords ?? [])].some((value) =>
      value.toLocaleLowerCase().includes(search),
    ),
  );
  return matches.sort(
    (a, b) =>
      Number(b.id.toLocaleLowerCase() === search || b.label.toLocaleLowerCase() === search) -
      Number(a.id.toLocaleLowerCase() === search || a.label.toLocaleLowerCase() === search),
  );
}
