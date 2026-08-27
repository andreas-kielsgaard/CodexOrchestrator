use super::domain::{
    CleanupStorageKey, DomainError, OperationAttemptId, RepositoryId, ReviewBuildId,
};
use std::path::{Component, Path};

/// The one durable projection from Review identity to an AppData build-attempt directory.
pub(crate) fn attempt_storage_key(
    repository_id: &RepositoryId,
    build_id: &ReviewBuildId,
    attempt_id: &OperationAttemptId,
) -> Result<CleanupStorageKey, DomainError> {
    for value in [
        repository_id.as_str(),
        build_id.as_str(),
        attempt_id.as_str(),
    ] {
        let mut components = Path::new(value).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            return Err(DomainError::new(
                "build storage identities must each be one path component",
            ));
        }
    }
    CleanupStorageKey::new(format!(
        "repositories/{}/build-output/{}/{}",
        repository_id.as_str(),
        build_id.as_str(),
        attempt_id.as_str()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_storage_is_derived_only_from_durable_identity() {
        let repository = RepositoryId::new("repository").unwrap();
        let build = ReviewBuildId::new("build").unwrap();
        let attempt = OperationAttemptId::new("attempt").unwrap();

        assert_eq!(
            attempt_storage_key(&repository, &build, &attempt)
                .unwrap()
                .as_str(),
            "repositories/repository/build-output/build/attempt"
        );
        assert!(attempt_storage_key(
            &RepositoryId::new("../repository").unwrap(),
            &build,
            &attempt
        )
        .is_err());
    }
}
