use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IdentityDomainError {
    EmptyIdentityId,
    EmptyDisplayName,
    InvalidColor,
}

impl fmt::Display for IdentityDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyIdentityId => "identity ID must be a non-empty opaque identifier",
            Self::EmptyDisplayName => "identity display name cannot be empty",
            Self::InvalidColor => "identity color must be a six-digit hexadecimal color",
        })
    }
}

impl Error for IdentityDomainError {}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct IdentityId(String);

impl IdentityId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, IdentityDomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IdentityDomainError::EmptyIdentityId);
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdentityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IdentityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IdentityShape {
    Circle,
    Square,
    Hexagon,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct IdentityDefinition {
    pub(crate) id: IdentityId,
    pub(crate) display_name: String,
    pub(crate) color: String,
    pub(crate) shape: IdentityShape,
}

impl IdentityDefinition {
    pub(crate) fn new(
        id: IdentityId,
        display_name: impl Into<String>,
        color: impl Into<String>,
        shape: IdentityShape,
    ) -> Result<Self, IdentityDomainError> {
        let definition = Self {
            id,
            display_name: display_name.into(),
            color: color.into(),
            shape,
        };
        definition.validate()?;
        Ok(definition)
    }

    pub(crate) fn assign(&self) -> AssignedAgentIdentity {
        AssignedAgentIdentity {
            origin_identity_id: Some(self.id.clone()),
            display_name: self.display_name.clone(),
            color: self.color.clone(),
            shape: self.shape,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), IdentityDomainError> {
        validate_identity_value(&self.display_name, &self.color)
    }
}

/// Session-owned identity snapshot. Later edits to its originating definition do not change it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssignedAgentIdentity {
    pub(crate) origin_identity_id: Option<IdentityId>,
    pub(crate) display_name: String,
    pub(crate) color: String,
    pub(crate) shape: IdentityShape,
}

impl AssignedAgentIdentity {
    pub(crate) fn new(
        origin_identity_id: Option<IdentityId>,
        display_name: impl Into<String>,
        color: impl Into<String>,
        shape: IdentityShape,
    ) -> Result<Self, IdentityDomainError> {
        let identity = Self {
            origin_identity_id,
            display_name: display_name.into(),
            color: color.into(),
            shape,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub(crate) fn validate(&self) -> Result<(), IdentityDomainError> {
        validate_identity_value(&self.display_name, &self.color)
    }
}

fn validate_identity_value(display_name: &str, color: &str) -> Result<(), IdentityDomainError> {
    if display_name.trim().is_empty() {
        return Err(IdentityDomainError::EmptyDisplayName);
    }
    if !is_hex_color(color) {
        return Err(IdentityDomainError::InvalidColor);
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..]
            .iter()
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_is_a_session_owned_snapshot() {
        let mut definition = IdentityDefinition::new(
            IdentityId::new("avery").unwrap(),
            "Avery",
            "#39745a",
            IdentityShape::Circle,
        )
        .unwrap();

        let assigned = definition.assign();
        definition.display_name = "Avery Updated".into();
        definition.color = "#112233".into();

        assert_eq!(assigned.display_name, "Avery");
        assert_eq!(assigned.color, "#39745a");
        assert_eq!(
            assigned.origin_identity_id.as_ref().map(IdentityId::as_str),
            Some("avery")
        );
    }

    #[test]
    fn assigned_identity_can_be_session_specific() {
        let assigned = AssignedAgentIdentity::new(
            None,
            "Session specialist",
            "#abcdef",
            IdentityShape::Hexagon,
        )
        .unwrap();

        assert!(assigned.origin_identity_id.is_none());
    }

    #[test]
    fn identity_values_reject_blank_names_and_non_hex_colors() {
        assert_eq!(
            AssignedAgentIdentity::new(None, " ", "#abcdef", IdentityShape::Square).unwrap_err(),
            IdentityDomainError::EmptyDisplayName
        );
        assert_eq!(
            AssignedAgentIdentity::new(None, "Avery", "green", IdentityShape::Square).unwrap_err(),
            IdentityDomainError::InvalidColor
        );
    }
}
