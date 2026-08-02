# Product and ad-hoc skill separation

Status: revised. Harness runtime loading remains product work.

## Observation

Product role skills and ad-hoc development skills originally shared `.agents/skills`, so ordinary Codex sessions received both catalogues. Wording alone could not prevent accidental selection. Later ad-hoc renaming separated vocabulary but still left 22 exposed workflow skills.

## Theory

Physical co-location made product roles appear usable before the product Harness supplied them. The oversized ad-hoc catalogue also resembled a second product role system rather than a small Codex coordination workflow.

## Revision

The seven product role skills remain under `.agents/product-skills`: Epic Plan Builder, Epic Bootstrap Generator, Epic Runner, Sprint Runner, Work Slice Planner, Work Unit Handler, and Work Unit Implementer. Their Harness availability is not claimed until the product implements and proves selective exposure.

Ordinary repository discovery now exposes three ad-hoc conversation-role skills and eight operation skills: Overall Plan, Slice Plan, and Plan Step. Their names and bodies use only that vocabulary and do not reference product roles or product-only capabilities.

`orchestration-skill-maintainer` remains an exposed maintenance utility outside the workflow catalogue. Historical reports and shared evidence are not workflow entry points.

## Evaluation

The storage boundary prevents ordinary Codex agents from discovering product role definitions. The compact role-and-operation catalogue also differentiates the ad-hoc workflow by structure and use, rather than relying only on renamed roles. Product Harness implementation remains outside this maintenance change.
