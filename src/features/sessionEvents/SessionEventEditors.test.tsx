import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { PromptSourceListEditor } from './PromptSourceListEditor';
import { TargetSelectionEditor } from './TargetSelectionEditor';
import { TriggerBindingEditor } from './TriggerBindingEditor';
import type { PromptSourceDefinition, SessionEventTriggerBinding, TargetSelection } from './types';

describe('Session Event definition editors', () => {
  it('switches trigger kinds and edits the concrete binding', () => {
    function Fixture() {
      const [value, setValue] = useState<SessionEventTriggerBinding>({ kind: 'user_request' });
      return <TriggerBindingEditor value={value} onChange={setValue} />;
    }

    render(<Fixture />);
    fireEvent.change(screen.getByLabelText('Trigger type'), {
      target: { value: 'application_event' },
    });
    const identity = screen.getByRole('group', { name: 'Application event kind' });
    const inputs = identity.querySelectorAll('input');
    fireEvent.change(inputs[2], { target: { value: 'workflow_activated' } });

    expect(inputs[0]).toHaveValue('application');
    expect(inputs[1]).toHaveValue('event_kind');
    expect(inputs[2]).toHaveValue('workflow_activated');
  });

  it('keeps prompt source ordering explicit and editable', () => {
    function Fixture() {
      const [value, setValue] = useState<PromptSourceDefinition[]>([
        { kind: 'literal', text: 'Connection context' },
        { kind: 'user_request_text' },
      ]);
      return <PromptSourceListEditor value={value} onChange={setValue} />;
    }

    render(<Fixture />);
    expect(
      screen
        .getAllByLabelText('Source type')
        .map((control) => (control as HTMLSelectElement).value),
    ).toEqual(['literal', 'user_request_text']);

    fireEvent.click(screen.getByRole('button', { name: 'Move prompt source 2 up' }));
    expect(
      screen
        .getAllByLabelText('Source type')
        .map((control) => (control as HTMLSelectElement).value),
    ).toEqual(['user_request_text', 'literal']);

    fireEvent.click(screen.getByRole('button', { name: 'Add prompt source' }));
    expect(screen.getAllByLabelText('Source type')).toHaveLength(3);
  });

  it('prevents create-on-missing when an exact Session is selected', () => {
    const initial: TargetSelection = {
      target: {
        kind: 'logical',
        address: {
          scope: { namespace: 'workflow', kind: 'instance', id: 'run-1' },
          subject: { namespace: 'workflow', kind: 'node', id: 'review' },
        },
      },
      cardinality: 'first',
      ordering: 'newest',
      running: 'any',
      createdBy: null,
      missing: 'create',
    };

    function Fixture() {
      const [value, setValue] = useState(initial);
      return <TargetSelectionEditor value={value} onChange={setValue} />;
    }

    render(<Fixture />);
    fireEvent.change(screen.getByLabelText('Address type'), { target: { value: 'exact' } });

    expect(screen.getByLabelText('If no Session matches')).toHaveValue('fail');
    expect(screen.getByRole('option', { name: 'Create a Session' })).toBeDisabled();
    expect(screen.getByText('Creating a Session requires a logical address.')).toBeVisible();
  });
});
