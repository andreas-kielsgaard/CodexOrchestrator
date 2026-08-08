# Owning catalogue boundary

## Observation and theory

The maintainer correctly considered reader context but did not treat storage namespace as part of that context. It therefore placed selectively exposed product definitions under a hidden `.agents` subdirectory: hidden from current automatic discovery, yet still apparently owned by the Codex agent catalogue.

## Revision

The maintainer now identifies the harness that owns exposure and stores definitions in that system's catalogue. Selectively supplied skills remain outside other harnesses' automatic discovery roots. The complete product catalogue moved to repository-owned `product/skills`.

## Evaluation

This is a general authoring boundary rather than a product-specific exception. It should prevent future skills from being placed according to the authoring agent's current environment instead of the intended reader's owning harness.
