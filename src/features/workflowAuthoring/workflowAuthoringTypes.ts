export type WorkflowEditorSelection = Readonly<{
  kind: 'node' | 'connection' | 'run';
  id: string | null;
}>;
