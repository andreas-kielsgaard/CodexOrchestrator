import { fireEvent, render, screen, within } from '@testing-library/react';
import { vi } from 'vitest';
import type { AssignedAgentIdentity } from '../../application/identities';
import { IdentityPickerDialog } from './IdentityPickerDialog';

const identity: AssignedAgentIdentity = {
  originIdentityId: 'identity-avery',
  displayName: 'Avery',
  color: '#39745a',
  shape: 'circle',
};

describe('IdentityPickerDialog', () => {
  it('returns a trimmed Session-owned identity with edited presentation', () => {
    const onSave = vi.fn();
    render(<IdentityPickerDialog identity={identity} onSave={onSave} onClose={vi.fn()} />);
    const dialog = screen.getByRole('dialog', { name: 'Edit identity' });

    fireEvent.change(within(dialog).getByLabelText('Identity display name'), {
      target: { value: '  Avery Stone  ' },
    });
    fireEvent.change(within(dialog).getByLabelText('Identity color'), {
      target: { value: '#2456aa' },
    });
    fireEvent.click(within(dialog).getByLabelText('Hexagon'));
    fireEvent.click(within(dialog).getByText('Apply identity'));

    expect(onSave).toHaveBeenCalledWith({
      originIdentityId: 'identity-avery',
      displayName: 'Avery Stone',
      color: '#2456aa',
      shape: 'hexagon',
    });
  });

  it('prevents empty names and closes on Escape', () => {
    const onClose = vi.fn();
    render(<IdentityPickerDialog identity={identity} onSave={vi.fn()} onClose={onClose} />);
    const dialog = screen.getByRole('dialog', { name: 'Edit identity' });

    fireEvent.change(within(dialog).getByLabelText('Identity display name'), {
      target: { value: '   ' },
    });
    expect(within(dialog).getByText('Apply identity')).toBeDisabled();

    fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });
});
