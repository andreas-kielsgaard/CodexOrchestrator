# Product and ad-hoc skill separation

Status: revised. Harness runtime loading remains product work.

## Observation

Product role skills and ad-hoc development skills originally shared `.agents/skills`, so ordinary Codex sessions received both catalogues. An intermediate move into a hidden `.agents` subdirectory stopped ordinary name discovery but still placed product definitions under the Codex agent namespace and falsely implied Codex ownership.

## Theory

Storage namespace communicates ownership as well as discoverability. A hidden subfolder under another harness's namespace remains the wrong source boundary even when current discovery happens not to recurse into it.

## Revision

The product catalogue now lives under repository-owned `product/skills`, outside Codex's `.agents` namespace and automatic skill roots. It contains Epic Plan Builder, Epic Bootstrap Generator, Epic Runner, Sprint Runner, Work Slice Planner, Work Unit Handler, Work Unit Implementer, and Route Epic Feedback. Their availability is established only when the product Harness selectively supplies them.

Ordinary repository discovery now exposes three ad-hoc conversation-role skills and eight operation skills: Overall Plan, Slice Plan, and Plan Step. Their names and bodies use only that vocabulary and do not reference product roles or product-only capabilities.

`orchestration-skill-maintainer` remains an exposed maintenance utility outside the workflow catalogue. Historical reports and shared evidence are not workflow entry points.

## Evaluation

The storage boundary now reflects both ownership and exposure: ordinary Codex agents discover the ad-hoc catalogue, while product definitions remain inert repository assets until the product Harness supplies them. Existing product code and documentation still naming the old path are intentionally outside this skill-maintenance correction.
