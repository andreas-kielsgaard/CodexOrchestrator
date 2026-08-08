import { fireEvent, render, screen } from '@testing-library/react';
import { OperationalSpineCheckpointDemo } from './OperationalSpineCheckpointDemo';

describe('OperationalSpineCheckpointDemo', () => {
  it('walks the accepted Work Unit path without crossing the dependent-activation boundary', () => {
    render(<OperationalSpineCheckpointDemo />);

    expect(screen.getByText('Planned')).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: '2. Implementation ready' }));
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Original Handler is application-ready.',
    );
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Implementer is application-ready.',
    );

    fireEvent.click(screen.getByRole('button', { name: '3. Review ready' }));
    expect(screen.getByText('Ready for Handler review')).toBeVisible();
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Handler semantic judgment is pending',
    );

    fireEvent.click(screen.getByRole('button', { name: '4. Handler accepted' }));
    expect(screen.getByLabelText('Work Unit context')).toHaveTextContent(
      'Handler decision: accepted',
    );

    fireEvent.click(screen.getByRole('button', { name: '5. Integrated and settled' }));
    const detail = screen.getByLabelText('Work Unit context');
    expect(detail).toHaveTextContent('Integration success was recorded');
    expect(detail).toHaveTextContent('Work Unit settlement was recorded');
    expect(detail).toHaveTextContent('for 2 dependent Work Units');
    expect(detail).toHaveTextContent('This does not activate dependent work');
  });
});
