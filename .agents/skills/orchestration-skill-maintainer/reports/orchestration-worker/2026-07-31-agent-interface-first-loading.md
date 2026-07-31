# Agent-interface-first loading

## Observation

Implementation workers may encounter UI setup or validation only after their role skill is already active. Catalog metadata influenced a fresh worker's reasoning but did not reliably demonstrate skill ingestion.

## Revision

Added one loading boundary before the worker's execution rules: load `$agent-interface-first` when implementation or validation could involve visible UI control.

## Evaluation

The hook addresses the worker's immediate tool choice without copying interface policy into the role or changing delegated scope.
