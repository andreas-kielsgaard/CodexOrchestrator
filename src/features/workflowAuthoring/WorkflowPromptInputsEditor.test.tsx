import { render, screen } from '@testing-library/react';
import { WorkflowPromptInputsEditor } from './WorkflowPromptInputsEditor';
import { repairRecipe } from './testFixtures';

it('presents file associations as distinct prompt sources without generated headlines', () => {
  const recipe = repairRecipe();
  render(
    <WorkflowPromptInputsEditor
      nodes={recipe.draft.nodes}
      output={{
        id: 'completed',
        name: 'Completed',
        kind: 'data',
        schema: { type: 'object', properties: { output: { type: 'string' } } },
      }}
      value={[
        { kind: 'output_field', field: 'output' },
        { kind: 'node_files', nodeId: 'author', association: 'edited' },
        { kind: 'node_files', nodeId: 'author', association: 'created' },
        { kind: 'node_files', nodeId: 'author', association: 'either' },
      ]}
      onChange={vi.fn()}
    />,
  );

  expect(
    screen.getAllByLabelText('Source type').map((control) => (control as HTMLSelectElement).value),
  ).toEqual(['output_field', 'node_files_edited', 'node_files_created', 'node_files_either']);
  expect(screen.getByLabelText('Field')).toHaveValue('output');
  expect(screen.queryByText(/1\. Source Node Output/)).toBeNull();
  expect(screen.queryByLabelText('File association')).toBeNull();
});
