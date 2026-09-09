use super::RuntimeSelections;

/// Null means inherit. This operation is shared by node and standalone creation.
pub(crate) fn overlay(
    inherited: &RuntimeSelections,
    explicit: &RuntimeSelections,
) -> RuntimeSelections {
    RuntimeSelections {
        model: explicit.model.clone().or_else(|| inherited.model.clone()),
        reasoning_mode: explicit
            .reasoning_mode
            .clone()
            .or_else(|| inherited.reasoning_mode.clone()),
        sandbox_mode: explicit.sandbox_mode.or(inherited.sandbox_mode),
    }
}
