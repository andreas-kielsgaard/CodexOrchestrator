import { fireEvent, render, screen, within } from '@testing-library/react';
import { vi } from 'vitest';
import {
  createRecordedHarnessManagementSource,
  recordedHarnessInspectorSessionId,
} from '../../dev/conversationHarnesses/recordedHarnessInspectorSource';
import { HarnessEditor } from './HarnessEditor';

describe('HarnessEditor visual contract', () => {
  it('keeps the reviewed management hierarchy instead of presenting a generic field form', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    const { container } = render(
      <HarnessEditor read={read} onBack={vi.fn()} onCommand={vi.fn()} />,
    );

    expect(container.querySelector('.harness-editor')).toHaveAttribute(
      'data-harness-editor-layout',
      'reviewed-management',
    );
    expect(screen.getByLabelText('Harness Management controls')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Back to conversation' })).toBeVisible();
    expect(screen.getByLabelText('Viewed harness version')).toHaveValue('version:3');
    expect(screen.getByRole('button', { name: 'Edit harness' })).toBeVisible();

    const sectionTitles = [
      ...container.querySelectorAll('.harness-management__card > header h2'),
    ].map((heading) => heading.textContent);
    expect(sectionTitles).toEqual([
      'Harness details',
      'Prompt prefix',
      'Skills',
      'Tools',
      'Models and reasoning',
      'Sandbox and authority',
      'Application hooks',
      'Version history',
    ]);
    expect(screen.getAllByRole('button', { name: /^Collapse / })).toHaveLength(8);

    expect(screen.getByRole('button', { name: /Permitted name pool/ })).toBeVisible();
    expect(screen.queryByLabelText('Machine key')).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit Agent color and shape' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit skills' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Edit tools' })).toBeVisible();
    expect(screen.getAllByRole('button', { name: /Always applicable/ })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: /Initial ingestion only/ })).toHaveLength(2);
    expect(
      screen.getByRole('slider', { name: 'Harness GPT-5.6 Terra minimum reasoning' }),
    ).toBeVisible();

    expect(screen.queryByLabelText('Harness skills')).toBeNull();
    expect(screen.queryByLabelText('Harness tools')).toBeNull();
    expect(screen.queryByLabelText('Allowed models')).toBeNull();
    expect(container.querySelector('.harness-definition-selector')).toBeNull();

    const promptToggle = screen.getByRole('button', { name: 'Collapse Prompt prefix' });
    fireEvent.click(promptToggle);
    expect(screen.getByRole('button', { name: 'Expand Prompt prefix' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.getByText(/You are the Epic Plan Builder/)).not.toBeVisible();
  });

  it('keeps catalog editing and item inspection in focused dialogs', async () => {
    const read = await createRecordedHarnessManagementSource().load({
      sessionId: recordedHarnessInspectorSessionId,
    });
    render(<HarnessEditor read={read} onBack={vi.fn()} onCommand={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /Permitted name pool/ }));
    const names = screen.getByRole('dialog', { name: 'Permitted name pool' });
    expect(within(names).getByLabelText('Search product names')).toBeVisible();
    expect(within(names).getByLabelText('Antoni Gaudi permitted')).toBeChecked();
    expect(within(names).getByLabelText('Grace Hopper permitted')).not.toBeChecked();
    fireEvent.click(within(names).getByRole('button', { name: 'Close permitted name pool' }));

    fireEvent.click(screen.getByRole('button', { name: 'Edit Agent color and shape' }));
    const identity = screen.getByRole('dialog', { name: 'Current Agent identity' });
    expect(within(identity).getByLabelText('Agent identity color')).toHaveValue('#39745a');
    expect(within(identity).getByRole('radio', { name: 'circle' })).toBeChecked();
    expect(within(identity).getByRole('radio', { name: 'square' })).not.toBeChecked();
    expect(within(identity).getByRole('radio', { name: 'hexagon' })).not.toBeChecked();
    fireEvent.click(within(identity).getByRole('button', { name: 'Close current Agent identity' }));

    fireEvent.click(screen.getByRole('button', { name: 'Edit skills' }));
    const skills = screen.getByRole('dialog', { name: 'Edit skills' });
    expect(within(skills).getByLabelText('Search all skills')).toBeVisible();
    expect(within(skills).getByRole('heading', { name: 'Selected skills' })).toBeVisible();
    expect(within(skills).getByRole('heading', { name: 'Skill catalog' })).toBeVisible();
  });
});
