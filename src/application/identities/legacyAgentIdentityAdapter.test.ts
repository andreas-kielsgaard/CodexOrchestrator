import type { AgentIdentity } from '../agentSessions';
import {
  assignedIdentityFromLegacyAgentIdentity,
  legacyHarnessRoleLabel,
} from './legacyAgentIdentityAdapter';

describe('legacy Agent identity adapter', () => {
  it('maps the wire-shaped value without treating its visual token as an Identity ID', () => {
    const legacy: AgentIdentity = {
      name: 'Avery',
      harnessRole: 'epic_plan_builder',
      visualIdentityToken: 'sunflower',
      visualIdentityAccent: '#39745a',
      visualIdentityShape: 'hexagon',
    };

    expect(assignedIdentityFromLegacyAgentIdentity(legacy)).toEqual({
      originIdentityId: null,
      displayName: 'Avery',
      color: '#39745a',
      shape: 'hexagon',
    });
    expect(legacyHarnessRoleLabel(legacy.harnessRole)).toBe('Epic Plan Builder');
  });

  it('supplies only transitional presentation defaults for older payloads', () => {
    expect(
      assignedIdentityFromLegacyAgentIdentity({
        name: 'Avery',
        harnessRole: 'planner',
        visualIdentityToken: 'drafting_compass',
      }),
    ).toMatchObject({ color: '#e8ece8', shape: 'circle' });
  });
});
