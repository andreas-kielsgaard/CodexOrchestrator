# Session branch layout and composer

## Chosen shape

- Group repository Sessions by repository and branch, combining instantiated worktrees across devices. Keep workflow and pinned sections intact.
- Branch inventory owns background discovery and caching; navigation loading never waits for remote discovery. Read creation timestamps from host worktree metadata for default newest-first order.
- A focused branch-layout projection owns colors, membership, detached rows, and mixed branch/row ordering. Reuse SQLite navigation orders and pointer gestures; sidebar drag operations never retarget execution.
- Permit regrouping only into the conversation's own branch. Ungrouped conversations can be ordered above or between branch groups.
- Extract caret-local slash parsing and preferred-match selection. Preserve surrounding draft text and current menu sorting.
- A focused autosize hook observes the session viewport and input, capped at two-thirds of its height. Match composer surround to the transcript background.

## Ownership

- Adapt session navigation DTOs/service, existing order validation, model/view, drag hook, branch header and session rows.
- Add branch inventory and branch layout modules rather than embedding discovery or placement rules in the screen.
- Retain existing workflow organization, context menus, attachment preview, and runtime selectors.
- Adapt the quick-menu hook and add parser/ranking tests; add composer sizing tests.

## Verification and publishing

Test inventory, grouping/order/drag restrictions, inline command selection and draft preservation, and sizing. Build an isolated worktree app and exercise these flows through the owned CLI inspector. Commit verified source, merge current main preserving unrelated changes, rebuild and publish the executable with a backup.
