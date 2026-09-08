import { render } from '@testing-library/react';
import { AgentIdentityMarker } from './AgentIdentityMarker';

describe('legacy AgentIdentityMarker', () => {
  it('preserves legacy metadata while delegating visual rendering to identities', () => {
    const { container } = render(
      <AgentIdentityMarker
        identity={{
          name: 'Avery Stone',
          harnessRole: 'epic_plan_builder',
          visualIdentityToken: 'sunflower',
          visualIdentityAccent: '#39745a',
          visualIdentityShape: 'hexagon',
        }}
      />,
    );

    const wrapper = container.querySelector('.agent-identity-marker.is-hexagon');
    expect(wrapper).toHaveAttribute('data-harness-role', 'epic_plan_builder');
    expect(wrapper).toHaveAttribute('data-visual-identity-token', 'sunflower');
    expect(wrapper).toHaveStyle({ '--agent-identity-accent': '#39745a' });
    expect(wrapper?.querySelector('.identity-marker.is-hexagon')).toHaveTextContent('AS');
  });
});
