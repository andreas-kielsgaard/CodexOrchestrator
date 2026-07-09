use super::*;

pub(crate) fn load_dashboard_snapshot(conn: &Connection) -> Result<TaskDashboardSnapshot, String> {
    let tasks = select_tasks(conn)?;
    let projects = select_projects(conn)?;
    let repos = select_repos(conn)?;
    let branches = select_branches(conn)?;
    let worktrees = select_worktrees(conn)?;
    let worktree_anchors = select_dashboard_worktree_anchors(conn)?;

    let mut groups = DASHBOARD_GROUPS
        .iter()
        .map(|(id, title)| DashboardGroup {
            id: *id,
            title: *title,
            tasks: Vec::new(),
        })
        .collect::<Vec<_>>();

    for task in tasks {
        if is_closed_task(&task) {
            continue;
        }

        let group_id = dashboard_group_id(&task);
        let group = groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| format!("Unknown dashboard group: {group_id}"))?;
        let project = projects
            .iter()
            .find(|project| project.id == task.project_id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Unassigned project".to_string());
        let repo = task
            .repo_id
            .as_ref()
            .and_then(|id| repos.iter().find(|repo| repo.id == *id))
            .map(|repo| repo.name.clone());
        let branch = task
            .branch_id
            .as_ref()
            .and_then(|id| branches.iter().find(|branch| branch.id == *id))
            .map(|branch| branch.name.clone());
        let worktree_path = task
            .worktree_id
            .as_ref()
            .and_then(|id| worktrees.iter().find(|worktree| worktree.id == *id))
            .map(|worktree| worktree.path.clone());

        group.tasks.push(DashboardTask {
            id: task.id,
            title: task.title,
            summary: task.summary,
            project,
            execution_state: task.execution_state,
            attention_state: task.attention_state,
            priority: task.priority,
            repo,
            branch,
            worktree_path,
            updated_at: task.updated_at,
        });
    }

    for group in &mut groups {
        group
            .tasks
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    let mut dashboard_projects = projects
        .iter()
        .map(|project| TaskDashboardProject {
            id: project.id.clone(),
            name: project.name.clone(),
        })
        .collect::<Vec<_>>();
    dashboard_projects.sort_by(|left, right| left.name.cmp(&right.name));

    let mut dashboard_repos = repos
        .into_iter()
        .map(|repo| {
            let project = projects
                .iter()
                .find(|project| project.id == repo.project_id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "Unassigned project".to_string());

            TaskDashboardRepo {
                id: repo.id,
                project_id: repo.project_id,
                project,
                name: repo.name,
                root_path: repo.root_path,
            }
        })
        .collect::<Vec<_>>();
    dashboard_repos.sort_by(|left, right| {
        left.project
            .cmp(&right.project)
            .then_with(|| left.name.cmp(&right.name))
    });
    let total_open_tasks = groups.iter().map(|group| group.tasks.len()).sum();

    Ok(TaskDashboardSnapshot {
        groups,
        projects: dashboard_projects,
        repos: dashboard_repos,
        worktree_anchors,
        total_open_tasks,
    })
}

pub(crate) fn dashboard_group_id(task: &TaskRow) -> &'static str {
    if task.attention_state == "needs_action_now" {
        return "needs_action_now";
    }

    if task.attention_state == "needs_review" {
        return "review_decide";
    }

    if task.execution_state == "running" || task.execution_state == "queued" {
        return "working";
    }

    if task.attention_state == "waiting_on_agent" || task.attention_state == "waiting_on_external" {
        return "waiting";
    }

    "later"
}

pub(crate) fn is_closed_task(task: &TaskRow) -> bool {
    task.execution_state == "archived" || task.execution_state == "abandoned"
}
