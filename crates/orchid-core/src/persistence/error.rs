use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistencePhase {
    Open,
    Configure,
    NestedOperation,
    Lock,
    Begin,
    Rollback,
    Commit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistenceError {
    operation: &'static str,
    phase: PersistencePhase,
    message: String,
}

impl PersistenceError {
    pub fn new(
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
    pub fn phase(&self) -> PersistencePhase {
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
pub enum ManagedOperationError<E> {
    Infrastructure(PersistenceError),
    Domain(E),
}

impl<E: std::fmt::Display> std::fmt::Display for ManagedOperationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infrastructure(error) => error.fmt(f),
            Self::Domain(error) => error.fmt(f),
        }
    }
}

impl ManagedOperationError<String> {
    pub fn into_string(self) -> String {
        match self {
            Self::Infrastructure(error) => error.to_string(),
            Self::Domain(error) => error,
        }
    }
}
