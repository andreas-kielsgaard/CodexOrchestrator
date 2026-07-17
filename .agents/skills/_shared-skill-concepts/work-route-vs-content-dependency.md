# Work Route Vs Content Dependency

A work route is where a worker should act: repo path, branch, worktree, and reuse/create policy.

A content dependency is source material the slice actually depends on: a commit, ref, document, API state, or repo content needed for correctness.

Branch or worktree names are usually routes, not content locks. Make drift fatal only when the work truly depends on the exact ref identity. Otherwise continue from a clean route with the needed content and report drift as route context.
