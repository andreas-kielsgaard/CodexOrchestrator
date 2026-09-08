use super::domain::{
    CleanupStorageKey, DomainError, OperationAttemptId, RepositoryId, ReviewBuildId,
};
use sha2::{Digest, Sha256};
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
    let mut digest = Sha256::new();
    for value in [
        repository_id.as_str(),
        build_id.as_str(),
        attempt_id.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    let token = digest.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    CleanupStorageKey::new(format!("attempts/{token}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_storage_is_derived_only_from_durable_identity() {
        let repository = RepositoryId::new("repository").unwrap();
        let build = ReviewBuildId::new("build").unwrap();
        let attempt = OperationAttemptId::new("attempt").unwrap();

        let first = attempt_storage_key(&repository, &build, &attempt).unwrap();
        let second = attempt_storage_key(&repository, &build, &attempt).unwrap();
        assert_eq!(first, second);
        assert!(first.as_str().starts_with("attempts/"));
        assert_eq!(first.as_str().len(), "attempts/".len() + 32);
        assert!(!first.as_str().contains(repository.as_str()));
        assert_ne!(
            first,
            attempt_storage_key(
                &repository,
                &build,
                &OperationAttemptId::new("another-attempt").unwrap()
            )
            .unwrap()
        );
        assert!(attempt_storage_key(
            &RepositoryId::new("../repository").unwrap(),
            &build,
            &attempt
        )
        .is_err());
    }
}
