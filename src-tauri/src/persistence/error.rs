use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersistencePhase {
    Open,
    Configure,
    NestedOperation,
    Lock,
    Begin,
    Rollback,
    Commit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PersistenceError {
    operation: &'static str,
    phase: PersistencePhase,
    message: String,
}

impl PersistenceError {
    pub(crate) fn new(
        operation: &'static str,
        phase: PersistencePhase,
        message: impl Into<String>,
    ) -> Self {
        Self {
            operation,
            phase,
            message: message.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn phase(&self) -> PersistencePhase {
        self.phase
    }
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Unable to {} during {:?}: {}",
            self.operation, self.phase, self.message
        )
    }
}

impl Error for PersistenceError {}

#[derive(Debug)]
pub(crate) enum ManagedOperationError<E> {
    Infrastructure(PersistenceError),
    Domain(E),
}

impl ManagedOperationError<String> {
    pub(crate) fn into_string(self) -> String {
        match self {
            Self::Infrastructure(error) => error.to_string(),
            Self::Domain(error) => error,
        }
    }
}
