use super::domain::{
    ArtifactRelativePath, ArtifactSetId, ArtifactStorageKey, ContentHash, OperationAttemptId,
    ReviewBuildId, VerifiedArtifactFile, VerifiedArtifactSet,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub(crate) struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub(crate) fn open(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root)
            .map_err(|error| format!("create Worktree Review artifact storage: {error}"))?;
        let root = root
            .canonicalize()
            .map_err(|error| format!("resolve Worktree Review artifact storage: {error}"))?;
        Ok(Self { root })
    }

    pub(crate) fn promote(
        &self,
        build_id: ReviewBuildId,
        attempt_id: OperationAttemptId,
        source_root: &Path,
        declared_files: &[PathBuf],
    ) -> Result<VerifiedArtifactSet, String> {
        if declared_files.is_empty() {
            return Err("A successful build did not declare any distributable artifacts.".into());
        }
        let source_root = source_root
            .canonicalize()
            .map_err(|_| "The build output root is unavailable.".to_string())?;
        let artifact_set_id = ArtifactSetId::random();
        let staging_parent = self.root.join("staging");
        let staging = staging_parent.join(artifact_set_id.as_str());
        let final_parent = self
            .root
            .join("artifacts")
            .join(build_id.as_str())
            .join(attempt_id.as_str());
        let final_root = final_parent.join(artifact_set_id.as_str());
        if staging.exists() || final_root.exists() {
            return Err("The artifact publication identity already exists.".into());
        }
        fs::create_dir_all(&staging)
            .map_err(|error| format!("create artifact staging directory: {error}"))?;

        let promoted = self.stage_files(&source_root, &staging, declared_files);
        let result = (|| {
            let files = promoted?;
            let manifest = serde_json::to_vec(&files)
                .map_err(|_| "The artifact manifest could not be encoded.".to_string())?;
            let manifest_hash = ContentHash::new(format!("{:x}", Sha256::digest(&manifest)))
                .map_err(|error| error.to_string())?;
            let mut manifest_file = fs::File::create(staging.join("manifest.json"))
                .map_err(|_| "The artifact manifest could not be created.".to_string())?;
            manifest_file
                .write_all(&manifest)
                .and_then(|_| manifest_file.sync_all())
                .map_err(|_| "The artifact manifest could not be persisted.".to_string())?;
            drop(manifest_file);
            fs::create_dir_all(&final_parent)
                .map_err(|_| "The artifact publication directory is unavailable.".to_string())?;
            fs::rename(&staging, &final_root).map_err(|_| {
                "The verified artifact set could not be published atomically.".to_string()
            })?;
            let storage_key = ArtifactStorageKey::new(format!(
                "artifacts/{}/{}/{}",
                build_id.as_str(),
                attempt_id.as_str(),
                artifact_set_id.as_str()
            ))
            .map_err(|error| error.to_string())?;
            let artifact_set = VerifiedArtifactSet {
                id: artifact_set_id,
                build_id,
                attempt_id,
                storage_key,
                manifest_hash,
                files,
                verified_at: Utc::now(),
            };
            artifact_set.validate().map_err(|error| error.to_string())?;
            Ok(artifact_set)
        })();
        if result.is_err() && staging.starts_with(&self.root) {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub(crate) fn resolve_file(
        &self,
        storage_key: &ArtifactStorageKey,
        relative_path: &ArtifactRelativePath,
    ) -> Result<(PathBuf, PathBuf), String> {
        let artifact_root = self
            .root
            .join(storage_key.as_str())
            .canonicalize()
            .map_err(|_| "The retained build output is unavailable.".to_string())?;
        if !artifact_root.is_dir() || !artifact_root.starts_with(&self.root) {
            return Err("The retained build output is outside Worktree Review storage.".into());
        }
        let file = artifact_root
            .join(relative_path.as_str())
            .canonicalize()
            .map_err(|_| "The retained application executable is unavailable.".to_string())?;
        if !file.is_file() || !file.starts_with(&artifact_root) {
            return Err("The retained application executable is invalid.".into());
        }
        Ok((artifact_root, file))
    }

    fn stage_files(
        &self,
        source_root: &Path,
        staging: &Path,
        declared_files: &[PathBuf],
    ) -> Result<Vec<VerifiedArtifactFile>, String> {
        let mut relative_files = declared_files.to_vec();
        relative_files.sort();
        relative_files.dedup();
        if relative_files.len() != declared_files.len() {
            return Err("The build declared the same artifact more than once.".into());
        }
        relative_files
            .into_iter()
            .map(|relative| {
                validate_relative(&relative)?;
                let source = source_root.join(&relative);
                let metadata = fs::symlink_metadata(&source)
                    .map_err(|_| "A declared build artifact is unavailable.".to_string())?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    return Err("A declared build artifact is not a regular file.".into());
                }
                let canonical_source = source
                    .canonicalize()
                    .map_err(|_| "A declared build artifact is unavailable.".to_string())?;
                if !canonical_source.starts_with(source_root) {
                    return Err("A declared build artifact escapes its output root.".into());
                }
                let destination = staging.join(&relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|_| "Artifact staging storage is unavailable.".to_string())?;
                }
                fs::copy(&canonical_source, &destination).map_err(|_| {
                    "A build artifact could not be copied into AppData.".to_string()
                })?;
                let (content_hash, bytes) = hash_file(&destination)?;
                Ok(VerifiedArtifactFile {
                    relative_path: ArtifactRelativePath::new(
                        relative.to_string_lossy().replace('\\', "/"),
                    )
                    .map_err(|error| error.to_string())?,
                    content_hash,
                    bytes,
                })
            })
            .collect()
    }
}

fn validate_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Artifact paths must be normalized paths below the build output root.".into());
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<(ContentHash, u64), String> {
    let mut file =
        fs::File::open(path).map_err(|_| "A staged build artifact is unavailable.".to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "A staged build artifact could not be verified.".to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        bytes = bytes.saturating_add(read as u64);
    }
    Ok((
        ContentHash::new(format!("{:x}", digest.finalize())).map_err(|error| error.to_string())?,
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotes_verified_files_into_an_immutable_appdata_location() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        fs::create_dir_all(output.join("nested")).unwrap();
        fs::write(output.join("nested/app.exe"), b"verified application").unwrap();
        let store = ArtifactStore::open(directory.path().join("appdata")).unwrap();

        let promoted = store
            .promote(
                ReviewBuildId::new("build-one").unwrap(),
                OperationAttemptId::new("attempt-one").unwrap(),
                &output,
                &[PathBuf::from("nested/app.exe")],
            )
            .unwrap();

        assert_eq!(promoted.files.len(), 1);
        assert_eq!(promoted.files[0].relative_path.as_str(), "nested/app.exe");
        assert_eq!(promoted.files[0].bytes, 20);
        let published = directory
            .path()
            .join("appdata")
            .join(promoted.storage_key.as_str());
        assert_eq!(
            fs::read(published.join("nested/app.exe")).unwrap(),
            b"verified application"
        );
        assert!(published.join("manifest.json").is_file());
    }

    #[test]
    fn rejects_escape_paths_before_copying() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        fs::create_dir_all(&output).unwrap();
        fs::write(directory.path().join("secret"), b"secret").unwrap();
        let store = ArtifactStore::open(directory.path().join("appdata")).unwrap();

        assert!(store
            .promote(
                ReviewBuildId::new("build-one").unwrap(),
                OperationAttemptId::new("attempt-one").unwrap(),
                &output,
                &[PathBuf::from("../secret")],
            )
            .is_err());
    }
}
