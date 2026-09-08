use super::domain::{
    HarnessDomainError, HarnessResolution, HarnessVersionRef, HarnessVersionReplacement,
};
use std::{collections::BTreeMap, error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HarnessResolutionError {
    InvalidReplacement(HarnessDomainError),
    RequestedVersionMissing(HarnessVersionRef),
    ReplacementTargetMissing {
        source: HarnessVersionRef,
        target: HarnessVersionRef,
    },
    DuplicateReplacementSource(HarnessVersionRef),
    ReplacementCycle(Vec<HarnessVersionRef>),
}

impl fmt::Display for HarnessResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReplacement(error) => error.fmt(formatter),
            Self::RequestedVersionMissing(reference) => {
                write!(
                    formatter,
                    "requested Harness version {reference} does not exist"
                )
            }
            Self::ReplacementTargetMissing { source, target } => write!(
                formatter,
                "Harness version replacement from {source} targets missing version {target}"
            ),
            Self::DuplicateReplacementSource(source) => write!(
                formatter,
                "more than one Harness version replacement starts at {source}"
            ),
            Self::ReplacementCycle(path) => {
                let path = path
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" -> ");
                write!(
                    formatter,
                    "Harness version replacement cycle detected: {path}"
                )
            }
        }
    }
}

impl Error for HarnessResolutionError {}

/// Resolves one exact version reference through an unambiguous replacement chain.
/// Replacement paths include the requested version and every traversed target.
pub(crate) fn resolve_harness_version(
    requested: &HarnessVersionRef,
    available_versions: &[HarnessVersionRef],
    replacements: &[HarnessVersionReplacement],
) -> Result<HarnessResolution, HarnessResolutionError> {
    let available = available_versions
        .iter()
        .cloned()
        .map(|reference| (reference, ()))
        .collect::<BTreeMap<_, _>>();
    if !available.contains_key(requested) {
        return Err(HarnessResolutionError::RequestedVersionMissing(
            requested.clone(),
        ));
    }

    let mut replacements_by_source = BTreeMap::new();
    for replacement in replacements {
        replacement
            .validate()
            .map_err(HarnessResolutionError::InvalidReplacement)?;
        if !available.contains_key(replacement.target()) {
            return Err(HarnessResolutionError::ReplacementTargetMissing {
                source: replacement.source().clone(),
                target: replacement.target().clone(),
            });
        }
        if replacements_by_source
            .insert(replacement.source().clone(), replacement.target().clone())
            .is_some()
        {
            return Err(HarnessResolutionError::DuplicateReplacementSource(
                replacement.source().clone(),
            ));
        }
    }

    let mut current = requested.clone();
    let mut path = vec![current.clone()];
    while let Some(target) = replacements_by_source.get(&current) {
        if let Some(cycle_start) = path.iter().position(|reference| reference == target) {
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(target.clone());
            return Err(HarnessResolutionError::ReplacementCycle(cycle));
        }
        current = target.clone();
        path.push(current.clone());
    }

    if path.len() == 1 {
        Ok(HarnessResolution::current(current))
    } else {
        Ok(HarnessResolution::replaced(
            requested.clone(),
            current,
            path,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::domain::{HarnessId, HarnessMigrationOutcome, HarnessVersionNumber};

    fn reference(harness_id: &str, version: u64) -> HarnessVersionRef {
        HarnessVersionRef::new(
            HarnessId::new(harness_id).unwrap(),
            HarnessVersionNumber::new(version).unwrap(),
        )
    }

    fn replacement(
        source: &HarnessVersionRef,
        target: &HarnessVersionRef,
    ) -> HarnessVersionReplacement {
        HarnessVersionReplacement::new(source.clone(), target.clone()).unwrap()
    }

    #[test]
    fn returns_current_when_no_replacement_is_ordered() {
        let requested = reference("epic-plan-builder", 1);

        let resolution = resolve_harness_version(&requested, &[requested.clone()], &[]).unwrap();

        assert_eq!(resolution.requested(), &requested);
        assert_eq!(resolution.resolved(), &requested);
        assert_eq!(resolution.outcome(), &HarnessMigrationOutcome::Current);
    }

    #[test]
    fn follows_a_deterministic_multi_hop_replacement_chain() {
        let version_1 = reference("epic-plan-builder", 1);
        let version_2 = reference("epic-plan-builder", 2);
        let version_3 = reference("epic-plan-builder", 3);
        let replacements = vec![
            replacement(&version_2, &version_3),
            replacement(&version_1, &version_2),
        ];

        let resolution = resolve_harness_version(
            &version_1,
            &[version_3.clone(), version_1.clone(), version_2.clone()],
            &replacements,
        )
        .unwrap();

        assert_eq!(resolution.requested(), &version_1);
        assert_eq!(resolution.resolved(), &version_3);
        assert!(resolution.outcome().was_replaced());
        assert_eq!(
            resolution.outcome(),
            &HarnessMigrationOutcome::ReplacementApplied {
                path: vec![version_1, version_2, version_3],
            }
        );
    }

    #[test]
    fn rejects_a_replacement_with_a_missing_target() {
        let version_1 = reference("epic-plan-builder", 1);
        let version_2 = reference("epic-plan-builder", 2);

        let error = resolve_harness_version(
            &version_1,
            &[version_1.clone()],
            &[replacement(&version_1, &version_2)],
        )
        .unwrap_err();

        assert_eq!(
            error,
            HarnessResolutionError::ReplacementTargetMissing {
                source: version_1,
                target: version_2,
            }
        );
    }

    #[test]
    fn rejects_a_replacement_cycle() {
        let version_1 = reference("epic-plan-builder", 1);
        let version_2 = reference("epic-plan-builder", 2);
        let replacements = vec![
            replacement(&version_1, &version_2),
            replacement(&version_2, &version_1),
        ];

        let error = resolve_harness_version(
            &version_1,
            &[version_1.clone(), version_2.clone()],
            &replacements,
        )
        .unwrap_err();

        assert_eq!(
            error,
            HarnessResolutionError::ReplacementCycle(
                vec![version_1.clone(), version_2, version_1,]
            )
        );
    }

    #[test]
    fn rejects_more_than_one_replacement_for_the_same_source() {
        let version_1 = reference("epic-plan-builder", 1);
        let version_2 = reference("epic-plan-builder", 2);
        let version_3 = reference("epic-plan-builder", 3);

        let error = resolve_harness_version(
            &version_1,
            &[version_1.clone(), version_2.clone(), version_3.clone()],
            &[
                replacement(&version_1, &version_2),
                replacement(&version_1, &version_3),
            ],
        )
        .unwrap_err();

        assert_eq!(
            error,
            HarnessResolutionError::DuplicateReplacementSource(version_1)
        );
    }
}
