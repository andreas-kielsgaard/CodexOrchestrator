# Design QA

## Source

- Approved visual: `C:\Users\user\.codex\generated_images\019fa398-6f66-76d2-b7b9-38bb2b4898c9\exec-49659c02-277b-450d-bdcf-a182ef3ed623.png`
- Implemented route: `http://127.0.0.1:43231/?integration-settlement-review`

## Current review state

- Verified viewport: 1239 x 986 CSS pixels
- Selected stage: Handler review
- Default right-hand tab: Session stream
- Parallel detail tab: Evidence

## Required fidelity surfaces

- Control bar remains above the Work Unit summary.
- Work Unit title, purpose, status, and technical identity are a distinct top strip.
- Chronological nested activity stages occupy the left column.
- Handler and Worker passages share one chronological Session stream on the right.
- Application events and relevant MCP calls are compact records nested under their owning Integration activity stage, not Session turns.
- Application-owned stage summaries are themselves nested under the preceding Worker or Handler stage; only agent-owned stages remain primary.
- Selecting an agent turn expands its full read-only output and steps inline in the Session stream without a composer.
- Selecting an application or MCP record expands its application-owned detail inside Integration activity.
- Hovering either side cross-highlights the related activity and passage.
- Dates and processing duration appear in activity and Session metadata without a raw timestamp column.
- Evidence is a peer tab that opens with no selection and reveals file-diff or test-execution detail only after a record click.
- Hovering or focusing linked evidence reveals its Implementer-chain relationship and highlights that activity.
- Unavailable evidence stays visible and disabled.
- At narrow width, activity and Session panes stack and scroll independently.

## Comparison history

1. Initial narrow comparison removed an unnecessary outer workspace scrollbar while preserving independently scrollable panes.
2. Human review requested a full read-only turn view, dated timestamps, processing durations, and clearer evidence ownership.
3. Continued review first placed application events and relevant MCP calls in the Session chronology, then corrected them into the owning Integration activity stages.
4. Continued review promoted evidence to a peer tab and added inspectable file-diff and test-execution details.
5. The current desktop check shows no horizontal overflow, browser warnings, or browser errors.
6. Final human correction nested every Application-type summary; the user then authorized this review slice to be considered approved.

## Validation

- Focused interaction suite: 5 tests passed.
- Scoped ESLint: passed.
- Vite production bundle: passed.
- Fresh semantic browser interactions: inline Session inspection, system-record separation, blank Evidence entry, file diff, and test result detail passed.
- Browser console warning/error check: zero entries.
- `git diff --check`: passed.
- Repository `npm run build` remains blocked by unrelated baseline TypeScript errors in `nativeQuery.ts` and `nativeQuery.test.ts`; the changed UI compiles in the successful Vite bundle.

## Result

Human review slice approved after the final nesting correction. This fixture remains demonstration support, not production implementation proof.
