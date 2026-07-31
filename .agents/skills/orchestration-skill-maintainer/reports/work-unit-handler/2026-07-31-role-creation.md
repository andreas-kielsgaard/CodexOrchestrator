# Work Unit Handler Role Creation

## Trigger and gap

The product hierarchy assigns one agent responsibility for instantiating, reviewing, correcting, and integrating a Work Unit. Existing `work-unit-review` is only a review helper, while `work-slice-delegation` belongs to the separate ad-hoc root-orchestration flow.

## Reader and revision

The new skill addresses one Handler created by a Work Slice Planner. It requests a Work Unit Implementer, evaluates returned work, owns correction cycles, integrates the accepted outcome through the supplied boundary, and returns settlement to its Planner. It does not edit the product itself.

## Evaluation

The role gives each Work Unit one durable owner without adding a separate reviewer level. Implementation, Handler acceptance, and integration remain distinct evidence states. The skill does not claim a current product harness.

## Validation

`quick_validate.py` passed. A fresh Handler routed even a tiny CSS correction to an Implementer, retained review and correction ownership, integrated only after acceptance, and returned settlement to its Planner.
