import { fireEvent, render, screen } from '@testing-library/react';
import { CollapsibleSection } from './CollapsibleSection';

describe('CollapsibleSection', () => {
  it('exposes and controls its content as one accessible disclosure', () => {
    render(
      <CollapsibleSection title="Runtime policy">
        <p>Policy content</p>
      </CollapsibleSection>,
    );

    const toggle = screen.getByRole('button', { name: 'Collapse Runtime policy' });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Policy content')).toBeVisible();

    fireEvent.click(toggle);
    expect(screen.getByRole('button', { name: 'Expand Runtime policy' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.getByText('Policy content')).not.toBeVisible();
  });
});
