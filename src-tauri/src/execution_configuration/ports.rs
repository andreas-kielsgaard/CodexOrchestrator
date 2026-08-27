use super::runtime_profile::RuntimeProfileSnapshot;
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedRuntimeProfileSourceError {
    message: String,
}

impl SelectedRuntimeProfileSourceError {
    pub(crate) fn unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SelectedRuntimeProfileSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SelectedRuntimeProfileSourceError {}

pub(crate) trait SelectedRuntimeProfileSource: Send + Sync {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError>;
}
