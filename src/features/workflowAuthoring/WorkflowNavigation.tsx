import { Plus } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type { WorkflowRecipeInstance } from '../../application/workflowInstances';
import type { WorkflowRecipeSummaryDto } from '../../application/workflowAuthoring';

export type WorkflowNavigationTab = 'designs' | 'instances';

export function WorkflowNavigation({
  summaries,
  instances,
  selectedRecipeId,
  selectedInstanceId,
  busy,
  canCreateInstance,
  onTabChange,
  onCreateDesign,
  onCreateInstance,
  onOpenRecipe,
  onOpenInstance,
}: {
  readonly summaries: readonly WorkflowRecipeSummaryDto[];
  readonly instances: readonly WorkflowRecipeInstance[];
  readonly selectedRecipeId: string | null;
  readonly selectedInstanceId: string | null;
  readonly busy: boolean;
  readonly canCreateInstance: boolean;
  onTabChange(tab: WorkflowNavigationTab): void;
  onCreateDesign(): void;
  onCreateInstance(): void;
  onOpenRecipe(recipeId: string): void;
  onOpenInstance(instanceId: string): void;
}) {
  const [tab, setTab] = useState<WorkflowNavigationTab>(
    selectedInstanceId ? 'instances' : 'designs',
  );
  const [collapsedGroups, setCollapsedGroups] = useState<ReadonlySet<string>>(() => new Set());
  useEffect(() => {
    if (selectedInstanceId) {
      setTab('instances');
      onTabChange('instances');
    }
  }, [selectedInstanceId, onTabChange]);
  const selectTab = (next: WorkflowNavigationTab) => {
    setTab(next);
    onTabChange(next);
  };
  const groups = useMemo(() => {
    const byRecipe = new Map<string, WorkflowRecipeInstance[]>();
    for (const instance of instances) {
      const recipeId = instance.recipe.recipeId;
      byRecipe.set(recipeId, [...(byRecipe.get(recipeId) ?? []), instance]);
    }
    return [...byRecipe].map(([recipeId, values]) => ({
      recipeId,
      label:
        summaries.find((summary) => summary.recipeId === recipeId)?.name.trim() ||
        values[0]?.recipe.name.trim() ||
        'Untitled workflow',
      instances: values,
    }));
  }, [instances, summaries]);

  return (
    <aside className="workflow-authoring-screen__recipes">
      <header>
        <div>
          <p>Session Event recipes</p>
          <h1>Workflows</h1>
        </div>
      </header>
      <div className="workflow-navigation__tabs" role="tablist" aria-label="Workflow navigation">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'designs'}
          onClick={() => selectTab('designs')}
        >
          Designs
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'instances'}
          onClick={() => selectTab('instances')}
        >
          Instances
        </button>
      </div>
      {tab === 'designs' ? (
        <>
          <button
            className="workflow-navigation__create"
            type="button"
            disabled={busy}
            onClick={onCreateDesign}
          >
            <Plus size={15} aria-hidden="true" /> Create design
          </button>
          <nav aria-label="Workflow designs">
            {summaries.map((summary) => (
              <button
                type="button"
                className={selectedRecipeId === summary.recipeId ? 'is-selected' : undefined}
                key={summary.recipeId}
                onClick={() => onOpenRecipe(summary.recipeId)}
              >
                <strong>{summary.name.trim() || 'Untitled workflow'}</strong>
                <span>Draft v{summary.draftRevision}</span>
                <small>
                  {summary.activeRevision ? `Active v${summary.activeRevision}` : 'Not active'}
                </small>
              </button>
            ))}
          </nav>
        </>
      ) : (
        <section className="workflow-instance-list" aria-label="Workflow instances">
          <button
            className="workflow-navigation__create"
            type="button"
            disabled={busy || !canCreateInstance}
            onClick={onCreateInstance}
          >
            <Plus size={15} aria-hidden="true" /> Create instance
          </button>
          <div className="workflow-instance-list__groups">
            {groups.map((group) => (
              <details
                key={group.recipeId}
                open={!collapsedGroups.has(group.recipeId)}
                onToggle={(event) => {
                  const open = event.currentTarget.open;
                  setCollapsedGroups((current) => {
                    if (open && !current.has(group.recipeId)) return current;
                    if (!open && current.has(group.recipeId)) return current;
                    const next = new Set(current);
                    if (open) next.delete(group.recipeId);
                    else next.add(group.recipeId);
                    return next;
                  });
                }}
              >
                <summary>
                  <span>{group.label}</span>
                  <small>{group.instances.length}</small>
                </summary>
                <div>
                  {group.instances.map((instance) => (
                    <button
                      type="button"
                      key={instance.id}
                      aria-pressed={selectedInstanceId === instance.id}
                      onClick={() => onOpenInstance(instance.id)}
                    >
                      <strong>{instance.name}</strong>
                      <small>v{instance.recipe.revision}</small>
                    </button>
                  ))}
                </div>
              </details>
            ))}
          </div>
        </section>
      )}
    </aside>
  );
}
