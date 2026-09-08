import { render, screen } from '@testing-library/react';
import { AgentIdentityBadge } from './AgentIdentityBadge';

describe('legacy AgentIdentityBadge', () => {
  it('keeps wire metadata on its boundary and delegates accessible content', () => {
    const { container } = render(
      <AgentIdentityBadge
        identity={{
          name: 'Avery',
          harnessRole: 'epic_plan_builder',
          visualIdentityToken: 'sunflower',
          visualIdentityAccent: '#39745a',
          visualIdentityShape: 'square',
        }}
      />,
    );

    expect(screen.getByLabelText('Avery, Epic Plan Builder')).toBeVisible();
    expect(container.querySelector('.agent-identity-badge')).toHaveAttribute(
      'data-visual-identity-token',
      'sunflower',
    );
    expect(container.querySelector('.identity-marker.is-square')).toHaveTextContent('A');
  });
});
