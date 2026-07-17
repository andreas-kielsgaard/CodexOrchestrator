import { fireEvent, render, screen } from '@testing-library/react';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';
import { App } from './App';

describe('App Harness Inspector development surface', () => {
  it('mounts the bounded inspector as an in-app development tab', async () => {
    render(
      <App
        {...createRecordedDevelopmentApplicationComposition({
          initialSurface: 'harness-inspector',
        })}
      />,
    );

    expect(screen.getByRole('button', { name: 'Harness Inspector' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('main', { name: 'Harness Inspector development' })).toBeVisible();
    expect(
      await screen.findByRole('heading', { name: 'Epic Plan Builder exploration' }),
    ).toBeVisible();
    expect(screen.getByText(/do not prove a live harness query or mutation path/i)).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Inspect harness' }));
    expect(await screen.findByRole('heading', { name: 'Epic Plan Builder' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    expect(await screen.findByLabelText('Agent Sessions')).toBeVisible();
  });
});
