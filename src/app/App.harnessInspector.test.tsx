import { fireEvent, render, screen, within } from '@testing-library/react';
import { createRecordedDevelopmentApplicationComposition } from '../dev/orchestrationSection/recordedOrchestrationClient';
import { App } from './App';

describe('App Harness Management preview', () => {
  it('mounts the recorded management view without a duplicate page title', async () => {
    render(
      <App
        {...createRecordedDevelopmentApplicationComposition({
          initialSurface: 'harness-inspector',
        })}
      />,
    );

    expect(screen.getByRole('button', { name: 'Harness Management' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('main', { name: 'Harness Management preview' })).toBeVisible();
    expect(await screen.findByRole('heading', { name: /: Epic Plan Builder$/ })).toBeVisible();
    expect(screen.queryByText('Agent Session Harness Inspector')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Manage harness' }));
    expect(await screen.findByRole('region', { name: 'Harness Management' })).toBeVisible();
  });

  it('resolves the same recorded identity when the Session is reopened from Agent Sessions', async () => {
    render(
      <App
        {...createRecordedDevelopmentApplicationComposition({
          initialSurface: 'harness-inspector',
        })}
      />,
    );
    const heading = await screen.findByRole('heading', { name: /: Epic Plan Builder$/ });
    const displayTitle = heading.textContent;

    fireEvent.click(screen.getByRole('button', { name: 'Agent Sessions' }));
    const sessionList = await screen.findByRole('navigation', { name: 'Session list' });
    const recordedSession = within(sessionList).getByRole('button', {
      name: new RegExp(displayTitle ?? 'Epic Plan Builder'),
    });
    fireEvent.click(recordedSession);

    expect(await screen.findByRole('heading', { name: displayTitle ?? '' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Manage harness' })).toBeVisible();
  });
});
