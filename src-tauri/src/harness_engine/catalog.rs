use super::{
    configuration::{HarnessConfiguration, HarnessMetadata},
    domain::{HarnessId, HarnessVersionNumber, HarnessVersionRef, HarnessVersionScope},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessRecord {
    pub(crate) id: HarnessId,
    pub(crate) metadata: HarnessMetadata,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessVersion {
    pub(crate) reference: HarnessVersionRef,
    pub(crate) scope: HarnessVersionScope,
    pub(crate) configuration: HarnessConfiguration,
    pub(crate) configuration_digest: String,
    pub(crate) created_at: DateTime<Utc>,
}

impl HarnessVersion {
    pub(crate) fn build(
        reference: HarnessVersionRef,
        scope: HarnessVersionScope,
        configuration: HarnessConfiguration,
        created_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        configuration.validate()?;
        scope.validate().map_err(|error| error.to_string())?;
        let configuration_digest = configuration_digest(&configuration)?;
        Ok(Self {
            reference,
            scope,
            configuration,
            configuration_digest,
            created_at,
        })
    }

    pub(crate) fn verify(&self) -> Result<(), String> {
        self.configuration.validate()?;
        self.scope.validate().map_err(|error| error.to_string())?;
        let digest = configuration_digest(&self.configuration)?;
        if digest == self.configuration_digest {
            Ok(())
        } else {
            Err("Stored Harness configuration digest does not match its configuration.".into())
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDraft {
    pub(crate) harness_id: HarnessId,
    pub(crate) based_on_version: Option<HarnessVersionNumber>,
    pub(crate) configuration: HarnessConfiguration,
    pub(crate) draft_revision: u64,
    pub(crate) saved_at: DateTime<Utc>,
}

impl HarnessDraft {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.draft_revision == 0 {
            return Err("Harness draft revision must be greater than zero.".into());
        }
        self.configuration.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedHarnessVersion {
    pub(crate) requested: HarnessVersionRef,
    pub(crate) version: HarnessVersion,
    pub(crate) replacement_path: Vec<HarnessVersionRef>,
}

impl ResolvedHarnessVersion {
    pub(crate) fn was_replaced(&self) -> bool {
        self.requested != self.version.reference
    }
}

pub(crate) fn configuration_digest(configuration: &HarnessConfiguration) -> Result<String, String> {
    let bytes = canonical_json(configuration)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("Unable to encode Harness value: {error}"))
}

pub(crate) fn next_version(versions: &[HarnessVersion]) -> Result<HarnessVersionNumber, String> {
    let next = versions
        .iter()
        .map(|version| version.reference.version().get())
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "Harness version number overflowed.".to_string())?;
    HarnessVersionNumber::new(next).map_err(|error| error.to_string())
}
