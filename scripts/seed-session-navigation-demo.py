"""Populate an initialized, stopped navigation demo instance; no provider calls."""
import argparse
import json
import sqlite3
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path


def git(directory, *args):
    return subprocess.check_output(["git", "-C", str(directory), *args], text=True).strip()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("demo_directory", type=Path, help="Demo root containing app-data")
args = parser.parse_args()
demo = args.demo_directory.resolve()
database = demo / "app-data" / "codex-orchestrator-active-v3.sqlite"
if not database.is_file():
    parser.error("Initialize the app with this demo app-data directory, then stop it before seeding.")
repo = Path(__file__).resolve().parents[1]
connection = sqlite3.connect(database)
connection.execute("PRAGMA foreign_keys=ON")
existing = [row[0] for row in connection.execute("SELECT id FROM agent_sessions")]
if any(not sid.startswith(("smoke-", "demo-")) for sid in existing):
    parser.error("This database contains non-demo sessions; use a separate demo instance.")

repositories = [("repo-a", "Codex Orchestrator", repo)]
for repo_id, name, directory in [
    ("repo-b", "Empty project", demo / "empty-repository"),
    ("repo-c", "Story Game", demo / "story-repository"),
]:
    directory.mkdir(parents=True, exist_ok=True)
    if not (directory / ".git").exists():
        git(directory, "init", "-b", "main")
    repositories.append((repo_id, name, directory))

now = datetime(2026, 9, 14, 12, tzinfo=timezone.utc)
stamp = now.isoformat()
with connection:
    for sid in existing:
        connection.execute("DELETE FROM agent_sessions WHERE id=?", (sid,))
    for repo_id, name, directory in repositories:
        common = git(directory, "rev-parse", "--path-format=absolute", "--git-common-dir")
        connection.execute("INSERT INTO registered_repositories VALUES(?,?,?,?,?,?) ON CONFLICT(repository_id) DO UPDATE SET label=excluded.label,anchor_root=excluded.anchor_root,git_common_directory=excluded.git_common_directory", (repo_id, name, str(directory), common, stamp, stamp))

    target = {
        "repository": {"id": "repo-a", "name": "Codex Orchestrator", "gitCommonDirectory": git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir")},
        "branch": {"id": "demo-branch", "name": git(repo, "branch", "--show-current")},
        "worktree": {"id": "demo-worktree", "path": str(repo)},
    }
    roles = ["Planner", "Implementer", "Reviewer", "Tester", "Release writer"]
    nodes = [{
        "nodeId": f"node-{i}", "name": name, "positionX": i * 240, "positionY": 0,
        "capabilityProfileId": "demo-profile",
        "nodeProfile": {
            "contractVersion": 1,
            "allowedCapabilities": {"models": [], "reasoningModes": [], "sandboxModes": [], "mcpTools": {}, "skills": []},
            "pinnedDefaults": {"model": None, "reasoningMode": None, "sandboxMode": None},
        },
        "initialPrompt": None, "agentIdentityId": None,
    } for i, name in enumerate(roles)]
    for instance_id, name in [("flow-a", "Feature build"), ("flow-b", "Release preparation")]:
        record = {
            "id": instance_id, "name": name, "target": target, "createdAt": stamp,
            "recipe": {
                "contractVersion": 1, "recipeId": f"recipe-{instance_id}", "name": name,
                "revision": 1, "startingNodeId": "node-0", "nodes": nodes, "connections": [],
            },
        }
        connection.execute(
            "INSERT INTO workflow_recipe_instances VALUES(?,?) "
            "ON CONFLICT(id) DO UPDATE SET record_json=excluded.record_json",
            (instance_id, json.dumps(record)),
        )

    rows = [
        ("Navigation design notes", "repository", "repo-a", None),
        ("Compare the sidebar layouts", "repository", "repo-a", None),
        ("Polish session labels", "repository", "repo-a", None),
        ("Check keyboard navigation", "repository", "repo-a", None),
        ("Investigate a UI issue", "repository", "repo-a", None),
        ("Draft release notes", "repository", "repo-a", None),
        ("Review workspace defaults", "repository", "repo-a", None),
        ("Older repository discussion", "repository", "repo-a", None),
        ("Review the folder layout", "workflow_instance", "flow-a", None),
        ("Collect design feedback", "workflow_instance", "flow-a", None),
        ("Discuss the acceptance checks", "workflow_instance", "flow-a", None),
        *[(f"{role}: {task}", "default", None, ("flow-a", f"node-{i}"))
          for i, (role, task) in enumerate(zip(roles, [
              "implementation outline", "build the sidebar", "review the changes",
              "verify interactions", "document the outcome",
          ]))],
        ("Release discussion", "workflow_instance", "flow-b", None),
        ("Reviewer: release checklist", "default", None, ("flow-b", "node-2")),
        ("Plan the next chapter", "repository", "repo-c", None),
        ("Sketch an opening scene", "repository", "repo-c", None),
        ("Unfiled conversation", "unfiled", None, None),
        ("A quick question", "unfiled", None, None),
    ]
    for i, (title, kind, placement, owner) in enumerate(rows):
        session_id = f"demo-{i + 1:02d}"
        updated = (now - timedelta(minutes=i)).isoformat()
        cwd = repositories[2][2] if placement == "repo-c" else repo
        connection.execute("INSERT INTO agent_sessions(id,title,availability,working_directory,requested_options_json,created_at,updated_at) VALUES(?,?,'available',?,'{}',?,?)", (session_id, title, str(cwd), updated, updated))
        connection.execute("INSERT INTO agent_session_organization(session_id,placement_kind,placement_target_id,pinned_at) VALUES(?,?,?,?)", (session_id, kind, placement, updated if i < 6 else None))
        if owner:
            sequence = connection.execute("INSERT INTO agent_session_address_clock DEFAULT VALUES").lastrowid
            connection.execute("INSERT INTO agent_session_addresses(session_id,scope_namespace,scope_kind,scope_id,subject_namespace,subject_kind,subject_id,created_sequence) VALUES(?,'workflow','instance',?,'workflow','node',?,?)", (session_id, *owner, sequence))
    if connection.execute("SELECT 1 FROM sqlite_master WHERE name='session_navigation_order'").fetchone():
        connection.execute("DELETE FROM session_navigation_order")
connection.close()
print(f"Seeded {len(rows)} sessions, 3 repositories, 2 workflows, and 6 pins in {database}")
