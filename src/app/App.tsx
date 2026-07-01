import { Activity, CheckCircle2, Clock3, GitBranch, Inbox, PauseCircle } from 'lucide-react';
import { dashboardGroups } from '../domain/taskDashboard';

const groupIcons = {
  needs_action_now: Activity,
  review_decide: CheckCircle2,
  working: GitBranch,
  waiting: Clock3,
  later: PauseCircle,
};

export function App() {
  const totalOpenTasks = dashboardGroups.reduce((total, group) => total + group.tasks.length, 0);

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand-mark">CO</div>
        <nav>
          <a className="nav-item active" href="#open-tasks">
            <Inbox size={18} aria-hidden="true" />
            Open Tasks
          </a>
          <a className="nav-item" href="#projects">
            <GitBranch size={18} aria-hidden="true" />
            Projects
          </a>
        </nav>
      </aside>

      <section className="workspace" id="open-tasks">
        <header className="topbar">
          <div>
            <p className="eyebrow">Local-first control plane</p>
            <h1>Open Tasks</h1>
          </div>
          <div className="status-strip" aria-label="Dashboard totals">
            <span>{totalOpenTasks} open</span>
            <span>Codex adapters pending</span>
          </div>
        </header>

        <section className="dashboard-grid" aria-label="Open task groups">
          {dashboardGroups.map((group) => {
            const Icon = groupIcons[group.id];

            return (
              <article className="task-column" key={group.id}>
                <header className="column-header">
                  <div className="column-title">
                    <Icon size={18} aria-hidden="true" />
                    <h2>{group.title}</h2>
                  </div>
                  <span className="count">{group.tasks.length}</span>
                </header>

                <div className="task-list">
                  {group.tasks.map((task) => (
                    <section className="task-card" key={task.id}>
                      <div>
                        <h3>{task.title}</h3>
                        <p>{task.summary}</p>
                      </div>
                      <footer>
                        <span>{task.project}</span>
                        <span>{task.executionState}</span>
                      </footer>
                    </section>
                  ))}
                </div>
              </article>
            );
          })}
        </section>
      </section>
    </main>
  );
}
