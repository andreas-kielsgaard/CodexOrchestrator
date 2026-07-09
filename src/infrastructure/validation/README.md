# Validation Infrastructure

`src/infrastructure/validation` owns concrete validation-command execution
adapters.

Keep process spawning, output capture, exit-code handling, and runtime-specific
command execution details here. Application validation use cases should depend on
the validation runtime contract rather than process mechanics.
