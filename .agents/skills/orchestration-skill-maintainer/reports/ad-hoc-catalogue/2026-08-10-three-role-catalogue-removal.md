# Three-role ad-hoc catalogue removal

## Decision

The user reported that the ad-hoc workflow skills were interfering with ordinary Codex work and directed their deletion.

## Change

Removed the role and operation definitions for the Overall Plan, Plan Slice, and Plan Step conversations, including their discovery metadata. This removes twelve automatically exposed skills from the repository catalogue.

Preserved the skill-maintenance catalogue and reports, shared historical concepts, general Codex skills, and the product-owned definitions under `product/skills`.

## Effect

New Codex sessions in this repository will no longer discover the three-role ad-hoc workflow skills. Existing sessions may retain previously ingested instructions until their context is refreshed or replaced.
