use super::domain::{
    ReferenceIdentity, RunningFilter, SessionDirectoryEntry, SessionTarget, TargetCardinality,
    TargetOrdering, TargetSelection,
};
use super::ports::{SessionDirectory, SessionDirectoryError};
use std::cmp::Reverse;

pub(super) fn resolve_existing(
    directory: &dyn SessionDirectory,
    selection: &TargetSelection,
) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
    let mut entries = match &selection.target {
        SessionTarget::New { .. } => {
            return Err(SessionDirectoryError::new(
                "New Sessions must use explicit creation",
            ))
        }
        SessionTarget::Exact { session } => directory.find_exact(session)?.into_iter().collect(),
        SessionTarget::Logical { address } => directory.list_at_address(address)?,
    };

    entries.retain(|entry| running_matches(selection.running, entry.running));
    if let Some(filter) = &selection.created_by {
        entries.retain(|entry| {
            filter
                .event
                .as_ref()
                .map_or(true, |event| entry.created_by_event.as_ref() == Some(event))
                && filter.session.as_ref().map_or(true, |session| {
                    entry.created_by_session.as_ref() == Some(session)
                })
        });
    }

    entries.sort_by_key(|entry| ordering_key(selection.ordering, entry));
    if selection.cardinality == TargetCardinality::First {
        entries.truncate(1);
    }
    Ok(entries)
}

fn running_matches(filter: RunningFilter, running: bool) -> bool {
    match filter {
        RunningFilter::Any => true,
        RunningFilter::RunningOnly => running,
        RunningFilter::NotRunning => !running,
    }
}

fn ordering_key(
    ordering: TargetOrdering,
    entry: &SessionDirectoryEntry,
) -> (Reverse<Option<u64>>, Reverse<u64>, ReferenceIdentity) {
    let addressed = match ordering {
        TargetOrdering::Newest => None,
        TargetOrdering::LastAddressed => entry.last_addressed_sequence,
    };
    (
        Reverse(addressed),
        Reverse(entry.created_sequence),
        entry.session.clone(),
    )
}
