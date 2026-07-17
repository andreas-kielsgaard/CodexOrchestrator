/**
 * Application-owned intent for the managed Plan Builder Agent Session.
 * These requirements are declarative only; they do not authorize or configure a runtime.
 */
export interface ManagedPlanBuilderSessionConfiguration {
  readonly identity: 'epic_plan_builder';
  readonly role: 'epic_planning';
  readonly purpose: string;
  readonly implementedCapabilities: {
    readonly mcpTools: readonly ['get_epic_planning_context', 'save_epic_plan_proposal'];
    readonly durableProposal: 'pre_initiation';
  };
  titleForEpicName(epicName: string): string;
  readonly unsupportedCapabilities: readonly {
    readonly capability: 'skills' | 'orchestration_folder_routing';
    readonly status: 'unsupported';
  }[];
}

export const managedPlanBuilderSessionConfiguration: ManagedPlanBuilderSessionConfiguration = {
  identity: 'epic_plan_builder',
  role: 'epic_planning',
  purpose:
    'Develop and durably revise a pre-initiation Epic planning proposal through the shared Agent Session boundary.',
  implementedCapabilities: {
    mcpTools: ['get_epic_planning_context', 'save_epic_plan_proposal'],
    durableProposal: 'pre_initiation',
  },
  titleForEpicName(epicName) {
    const name = epicName.trim();
    return name ? `Epic builder session for ${name}` : 'Epic builder session';
  },
  unsupportedCapabilities: [{ capability: 'orchestration_folder_routing', status: 'unsupported' }],
};
