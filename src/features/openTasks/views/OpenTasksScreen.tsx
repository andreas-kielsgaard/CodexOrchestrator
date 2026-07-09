import { StartupScreen } from '../../../app/views/AppChrome';
import { ErrorNotice } from '../../../views/Notice';
import type {
  OpenTasksScreenActions,
  OpenTasksScreenViewModel,
} from '../viewModels/openTasksScreenViewModel';
import { OpenTaskReviewLayout } from './OpenTaskReviewLayout';
import { OpenTasksHeader } from './OpenTasksHeader';
import { RepoSetupForm } from './RepoSetupForm';
import { TaskComposerForm } from './TaskComposerForm';
import { TaskRunDetailPanel } from './TaskRunDetailPanel';

export interface OpenTasksScreenProps {
  view: OpenTasksScreenViewModel;
  actions: OpenTasksScreenActions;
}

export function OpenTasksScreen({ view, actions }: OpenTasksScreenProps) {
  if (!view.hasLoadedDashboard) {
    return (
      <StartupScreen
        loading={view.startup.loading}
        error={view.startup.error}
        onRetry={actions.retryStartup}
      />
    );
  }

  return (
      <section className="workspace" id="open-tasks">
        <OpenTasksHeader
          totalOpenTasks={view.header.totalOpenTasks}
          projectCount={view.header.projectCount}
          worktreeCount={view.header.worktreeCount}
          busy={view.header.busy}
          onReload={actions.reloadDashboard}
        />

        {view.error && <ErrorNotice error={view.error} />}

        <RepoSetupForm
          form={view.repoSetup.form}
          discoveredRepos={view.repoSetup.discoveredRepos}
          addBusy={view.repoSetup.addBusy}
          scanBusy={view.repoSetup.scanBusy}
          available={view.repoSetup.available}
          scanAvailable={view.repoSetup.scanAvailable}
          onChange={actions.changeRepoSetup}
          onSubmit={actions.submitRepoSetup}
          onScan={actions.scanRepos}
          onAddDiscovered={actions.addDiscoveredRepo}
        />

        <TaskComposerForm
          form={view.composer.form}
          projects={view.composer.projects}
          worktrees={view.composer.worktrees}
          busy={view.composer.busy}
          canCreate={view.composer.canCreate}
          onSubmit={actions.submitComposer}
          onSelectProject={actions.selectComposerProject}
          onSelectWorktree={actions.selectComposerWorktree}
          onChange={actions.changeComposer}
        />

        <OpenTaskReviewLayout
          review={view.review}
          editDraft={view.editDraft}
          detailPanel={
            <TaskRunDetailPanel
              detail={view.taskDetail}
              onClose={actions.closeTaskDetail}
              onReload={actions.reloadTaskDetail}
            />
          }
          onEditDraftChange={actions.changeEditDraft}
          onSaveEdit={actions.saveEdit}
          onCancelEdit={actions.cancelEdit}
          onPromptChange={actions.changePrompt}
          onStartRun={actions.startRun}
          onUpdateAttention={actions.updateAttention}
          onUpdateExecution={actions.updateExecution}
          onOpenDetail={actions.openDetail}
          onEditTask={actions.editTask}
          onArchiveTask={actions.archiveTask}
        />
      </section>
  );
}
