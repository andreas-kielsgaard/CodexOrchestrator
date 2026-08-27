import { render, screen } from '@testing-library/react';
import type { AssignedAgentIdentity } from '../../application/identities';
import { AgentIdentityBadge } from './AgentIdentityBadge';

const identity: AssignedAgentIdentity = {
  originIdentityId: 'identity-avery',
  displayName: 'Avery Stone',
  color: '#f4d35e',
  shape: 'hexagon',
};

describe('AgentIdentityBadge', () => {
  it('renders colored initials in the assigned shape and accepts caller context', () => {
    const { container } = render(
      <AgentIdentityBadge identity={identity} secondaryLabel="Plan builder" />,
    );

    expect(screen.getByLabelText('Avery Stone, Plan builder')).toBeVisible();
    expect(screen.getByText('AS')).toHaveClass('identity-marker', 'is-hexagon');
    expect(screen.getByText('AS')).toHaveStyle({
      '--identity-color': '#f4d35e',
      '--identity-foreground': '#17211b',
    });
    expect(container).toHaveTextContent('Plan builder');
  });

  it('keeps compact badges accessible without rendering duplicate text', () => {
    render(<AgentIdentityBadge identity={identity} compact />);

    expect(screen.getByLabelText('Avery Stone')).toHaveClass('is-compact');
    expect(screen.queryByText('Avery Stone')).toBeNull();
  });
});
