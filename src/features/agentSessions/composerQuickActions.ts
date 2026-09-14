import type { AgentSessionQuickFeatures } from '../../application/agentSessions/quickFeatures';
import type { PerMessageRuntimeSelection } from './PerMessageRuntimeControls';

export interface ComposerQuickFeatures {
  readonly contextKey: string;
  readonly load: () => Promise<AgentSessionQuickFeatures>;
  readonly selection: PerMessageRuntimeSelection;
  readonly setSelection: (selection: PerMessageRuntimeSelection) => void;
}

/** A command either opens choices or performs a local draft action. */
interface ComposerQuickActionDetails {
  readonly id: string;
  readonly label: string;
  readonly description: string;
  readonly keywords?: readonly string[];
  readonly disabledReason?: string;
  readonly selected?: boolean;
}
export type ComposerQuickAction = ComposerQuickActionDetails &
  (
    | { readonly children: readonly ComposerQuickAction[]; readonly run?: never }
    | {
        readonly children?: never;
        readonly run: () => { readonly replacement: string; readonly notice: string };
      }
  );

export function sessionQuickActions(
  capabilities: AgentSessionQuickFeatures,
  source: ComposerQuickFeatures,
  active: boolean,
): readonly ComposerQuickAction[] {
  const { selection, setSelection } = source;
  const effectiveModel = selection.model ?? capabilities.defaults.model;
  const model = capabilities.models.find((item) => item.id === effectiveModel);
  const effectiveReasoning =
    selection.reasoningMode ?? capabilities.defaults.reasoningMode ?? model?.defaultReasoningMode;
  const disabledReason = active ? 'Available after the current turn finishes' : undefined;
  const changeModel = (id: string | null) => {
    const next = capabilities.models.find(
      (item) => item.id === (id ?? capabilities.defaults.model),
    );
    const reasoningMode =
      next &&
      effectiveReasoning &&
      !next.reasoningModes.some((mode) => mode.id === effectiveReasoning)
        ? next.defaultReasoningMode
        : selection.reasoningMode;
    setSelection({ model: id, reasoningMode });
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
  const inherit = (field: 'model' | 'reasoningMode'): ComposerQuickAction => ({
    id: 'default',
    label: 'Use Session default',
    description: 'Apply the inherited choice to your next message',
    selected: selection[field] === null,
    disabledReason:
      field === 'reasoningMode' &&
      capabilities.defaults.reasoningMode &&
      model &&
      !model.reasoningModes.some((mode) => mode.id === capabilities.defaults.reasoningMode)
        ? 'The Session default is unavailable for this model'
        : undefined,
    run: () => {
      if (field === 'model') return changeModel(null);
      setSelection({ ...selection, [field]: null });
      return {
        replacement: '',
        notice: 'Reasoning: Session default',
      };
    },
  });
  return [
    {
      id: 'model',
      label: 'Model',
      description: effectiveModel ?? 'Choose a model for your next message',
      disabledReason,
      children: [
        inherit('model'),
        ...capabilities.models.map((item): ComposerQuickAction => ({
          id: item.id,
          label: item.label,
          description: item.description,
          selected: item.id === selection.model,
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
        inherit('reasoningMode'),
        ...(model?.reasoningModes ?? []).map((mode): ComposerQuickAction => ({
          id: mode.id,
          label: mode.id,
          description: mode.description,
          selected: mode.id === selection.reasoningMode,
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
