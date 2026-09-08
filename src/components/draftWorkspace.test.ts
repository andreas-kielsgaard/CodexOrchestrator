import { DraftWorkspace } from './draftWorkspace';

it('keeps newer edits across a save acknowledgement and navigation', () => {
  const workspace = new DraftWorkspace<{ name: string; revision: number }>();
  const initial = { name: 'Initial', revision: 1 };
  const submitted = { ...initial, name: 'Submitted' };
  const newer = { ...initial, name: 'Still typing' };
  workspace.load('a', initial);
  workspace.edit('a', submitted);
  workspace.edit('a', newer);
  const merge = (value: typeof initial, saved: typeof initial) => ({
    ...value,
    revision: saved.revision,
  });
  expect(workspace.acceptSave('a', submitted, { ...submitted, revision: 2 }, merge)).toEqual({
    ...newer,
    revision: 2,
  });
  expect(workspace.load('a', initial).name).toBe('Still typing');
  expect(workspace.dirty('a')).toBe(true);
  expect(workspace.restore('a', 'undo', merge)).toEqual({ ...submitted, revision: 2 });
  expect(workspace.dirty('a')).toBe(false);
  expect(workspace.restore('a', 'redo', merge)).toEqual({ ...newer, revision: 2 });
});
