import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SkillsPicker } from './SkillsPicker';

it('groups skills by provider and toggles a whole provider at once', async () => {
  const user = userEvent.setup();
  const change = vi.fn();
  render(
    <SkillsPicker
      catalog={{
        availability: 'available',
        options: [
          { value: 'job_agent:source_discovery', label: 'Source discovery' },
          { value: 'job_agent:source_registry', label: 'Source registry' },
          { value: 'workflow:review', label: 'Workflow review' },
        ],
      }}
      values={[]}
      onChange={change}
    />,
  );

  await user.click(screen.getByRole('button', { name: 'Set skills' }));
  await user.click(screen.getByRole('checkbox', { name: 'Toggle job_agent' }));
  await user.click(screen.getByRole('button', { name: 'Apply selection' }));

  expect(change).toHaveBeenCalledWith([
    'job_agent:source_discovery',
    'job_agent:source_registry',
  ]);
});
