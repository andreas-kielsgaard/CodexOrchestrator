use super::domain::{WorktreeApplicationError, WorktreeApplicationErrorKind};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const FILES: &[(&str, &str)] = &[
    (
        "build-tools.mjs",
        include_str!("../../../scripts/build-tools.mjs"),
    ),
    (
        "build/application.mjs",
        include_str!("../../../scripts/build/application.mjs"),
    ),
    (
        "build/cargo.mjs",
        include_str!("../../../scripts/build/cargo.mjs"),
    ),
    (
        "build/cargo-runner.mjs",
        include_str!("../../../scripts/build/cargo-runner.mjs"),
    ),
    (
        "build/process.mjs",
        include_str!("../../../scripts/build/process.mjs"),
    ),
];

pub(super) fn materialize() -> Result<PathBuf, WorktreeApplicationError> {
    let mut hash = Sha256::new();
    for (name, text) in FILES {
        hash.update(name.as_bytes());
        hash.update(text.as_bytes());
    }
    let key = format!("{:x}", hash.finalize());
    let root = std::env::temp_dir()
        .join("codex-orchestrator-build-tools")
        .join(&key[..16]);
    for (name, text) in FILES.iter().rev() {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).map_err(unavailable)?;
        if fs::read(&path).ok().as_deref() != Some(text.as_bytes()) {
            let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
            fs::write(&temp, text).map_err(unavailable)?;
            fs::rename(&temp, &path).map_err(unavailable)?;
        }
    }
    Ok(root.join("build-tools.mjs"))
}
fn unavailable(error: std::io::Error) -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::OutputUnavailable,
        format!("The application build tool could not be prepared: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_tool_is_complete_and_reusable_without_source_scripts() {
        let first = materialize().unwrap();
        assert_eq!(first, materialize().unwrap());
        for (name, text) in FILES {
            assert_eq!(
                fs::read_to_string(first.parent().unwrap().join(name)).unwrap(),
                *text
            );
        }
    }
}
