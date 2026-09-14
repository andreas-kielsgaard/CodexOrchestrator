export type WorkflowEditorSelection = Readonly<{
  kind: 'node' | 'connection';
  id: string | null;
}>;
