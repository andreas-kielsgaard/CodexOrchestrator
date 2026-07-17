import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { createRecordedApplicationTestModeComposition } from './recordedApplicationTestMode';
import { AgentTestModeScreen } from './AgentTestModeScreen';

describe('AgentTestModeScreen', () => {
  it('presents semantic controls, safe capture failure, and feedback delivery', async () => {
    const composition = createRecordedApplicationTestModeComposition();
    render(<AgentTestModeScreen {...composition} />);

    fireEvent.click(screen.getByRole('button', { name: 'Navigate to view' }));
    await screen.findByText('Navigated to the Agent Sessions workspace.');

    fireEvent.click(screen.getByRole('button', { name: 'Advance semantic action' }));
    await screen.findByText('Applied one recorded runtime step.');
    expect(screen.getByText(/eventCount=1/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Capture screenshot' }));
    await screen.findByText(/No app-window pixel capture adapter is connected/);

    fireEvent.change(screen.getByLabelText('Annotation'), {
      target: { value: 'Keep the processing state visible.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add annotation' }));
    await screen.findByText('Keep the processing state visible.');

    fireEvent.click(screen.getByRole('button', { name: 'Deliver recorded feedback' }));
    await waitFor(() => expect(composition.deliveredFeedback).toHaveLength(1));
    expect(screen.getByText(/"authority": "feedback_only"/)).toBeInTheDocument();
  });
});
