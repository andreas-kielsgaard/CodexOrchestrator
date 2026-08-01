use crate::worktree_runtime::{
    TestInstanceError, TestInstanceErrorKind, TestSourceRef, TestSourceResolver,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewWorktreeOption {
    pub(crate) source_ref: String,
    pub(crate) label: String,
    pub(crate) revision: String,
    pub(crate) compatibility: String,
    pub(crate) compatibility_message: String,
}

pub(crate) struct ReviewWorktreeCatalog {
    options: Vec<ReviewWorktreeOption>,
    paths: HashMap<String, PathBuf>,
    main_path: PathBuf,
}

impl ReviewWorktreeCatalog {
    pub(crate) fn discover(current_source: &Path, git: &Path) -> Result<Self, String> {
        let current_source = current_source
            .canonicalize()
            .map_err(|error| format!("resolve launcher source: {error}"))?;
        let output = Command::new(git)
            .arg("-C")
            .arg(&current_source)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .map_err(|error| format!("discover Git worktrees: {error}"))?;
        if !output.status.success() {
            return Err("Git worktree discovery failed".into());
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|_| "Git worktree discovery was not UTF-8".to_string())?;
        Self::from_porcelain(&text, &current_source)
    }

    fn from_porcelain(text: &str, current_source: &Path) -> Result<Self, String> {
        let mut options = Vec::new();
        let mut paths = HashMap::new();
        let mut main_path = None;
        for block in text.split("\n\n").filter(|block| !block.trim().is_empty()) {
            let mut path = None;
            let mut head = None;
            let mut branch = None;
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("worktree ") {
                    path = Some(PathBuf::from(value));
                } else if let Some(value) = line.strip_prefix("HEAD ") {
                    head = Some(value.to_owned());
                } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
                    branch = Some(value.to_owned());
                }
            }
            let path = path
                .ok_or_else(|| "Git returned a worktree without a path".to_string())?
                .canonicalize()
                .map_err(|error| format!("resolve discovered worktree: {error}"))?;
            if main_path.is_none() {
                main_path = Some(path.clone());
            }
            let head = head.ok_or_else(|| "Git returned a worktree without HEAD".to_string())?;
            let digest = format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()));
            let source_ref = format!("review-source-{}", &digest[..20]);
            let base = branch.unwrap_or_else(|| format!("Detached {}", &head[..head.len().min(8)]));
            let folder = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("worktree");
            let label = if path == current_source {
                format!("{base} - launcher source")
            } else {
                format!("{base} - {folder}")
            };
            let (compatibility, compatibility_message) = compatibility(&path);
            options.push(ReviewWorktreeOption {
                source_ref: source_ref.clone(),
                label,
                revision: head[..head.len().min(12)].to_owned(),
                compatibility,
                compatibility_message,
            });
            paths.insert(source_ref, path);
        }
        options.sort_by(|left, right| {
            let left_current = left.label.ends_with(" - launcher source");
            let right_current = right.label.ends_with(" - launcher source");
            right_current
                .cmp(&left_current)
                .then_with(|| left.label.cmp(&right.label))
        });
        if options.is_empty() {
            return Err("No Git worktrees were discovered".into());
        }
        Ok(Self {
            options,
            paths,
            main_path: main_path
                .ok_or_else(|| "No main Git worktree was discovered".to_string())?,
        })
    }

    pub(crate) fn options(&self) -> &[ReviewWorktreeOption] {
        &self.options
    }

    pub(crate) fn label(&self, source_ref: &str) -> Option<String> {
        self.options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .map(|option| option.label.clone())
    }

    pub(crate) fn ensure_compatible(&self, source_ref: &str) -> Result<(), String> {
        let option = self
            .options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        if option.compatibility == "compatible" {
            Ok(())
        } else {
            Err(option.compatibility_message.clone())
        }
    }

    pub(crate) fn compatibility(&self, source_ref: &str) -> (String, String) {
        self.options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .map(|option| {
                (
                    option.compatibility.clone(),
                    option.compatibility_message.clone(),
                )
            })
            .unwrap_or_else(|| {
                (
                    "incompatible".into(),
                    "The selected worktree is no longer available.".into(),
                )
            })
    }

    pub(super) fn scope(
        &self,
        source_ref: &str,
        name: String,
    ) -> Result<super::worktree_build::WorktreeScope, String> {
        let selected = self
            .paths
            .get(source_ref)
            .cloned()
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        Ok(super::worktree_build::WorktreeScope {
            name,
            selected,
            main: self.main_path.clone(),
        })
    }
}

fn compatibility(path: &Path) -> (String, String) {
    let marker = path.join("src-tauri/worktree-review-contract.json");
    let valid = fs::symlink_metadata(&marker)
        .ok()
        .filter(|metadata| metadata.file_type().is_file() && metadata.len() <= 4_096)
        .and_then(|_| fs::read(marker).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .is_some_and(|value| {
            value.get("version").and_then(serde_json::Value::as_u64) == Some(1)
                && value.get("readiness").and_then(serde_json::Value::as_str)
                    == Some("owned-window-and-rendered-application")
                && value.get("provenance").and_then(serde_json::Value::as_str)
                    == Some("worktree-build-details-v1")
        });
    if valid {
        (
            "compatible".into(),
            "This source declares the Worktree Review readiness and provenance contract.".into(),
        )
    } else {
        (
            "incompatible".into(),
            "This branch predates the Worktree Review child contract. Update it to a compatible lineage before Build or Open; waiting for a window cannot repair the missing readiness and provenance boundary.".into(),
        )
    }
}

impl TestSourceResolver for ReviewWorktreeCatalog {
    fn resolve(&self, source: &TestSourceRef) -> Result<PathBuf, TestInstanceError> {
        self.paths.get(source.as_str()).cloned().ok_or_else(|| {
            TestInstanceError::new(
                TestInstanceErrorKind::NotFound,
                "the selected review worktree is no longer available",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_catalog_exposes_labels_and_opaque_references_without_paths() {
        let directory = tempfile::tempdir().expect("directory");
        let current = directory.path().join("alpha");
        let other = directory.path().join("beta");
        std::fs::create_dir_all(&current).expect("current");
        std::fs::create_dir_all(&other).expect("other");
        let text = format!(
            "worktree {}\nHEAD 0123456789abcdef\nbranch refs/heads/codex/review\n\nworktree {}\nHEAD abcdef0123456789\ndetached\n",
            current.display(),
            other.display()
        );
        let catalog = ReviewWorktreeCatalog::from_porcelain(
            &text,
            &current.canonicalize().expect("canonical current"),
        )
        .expect("catalog");
        assert_eq!(catalog.options.len(), 2);
        assert!(catalog.options[0].source_ref.starts_with("review-source-"));
        let directory = directory.path().to_string_lossy();
        assert!(catalog
            .options
            .iter()
            .all(|option| !option.label.contains(directory.as_ref())));
        assert!(catalog
            .options
            .iter()
            .any(|option| option.label.contains("launcher source")));
        assert!(catalog.options[0].label.contains("launcher source"));
        assert_eq!(catalog.options[0].compatibility, "incompatible");
        assert!(catalog.options[0]
            .compatibility_message
            .contains("predates the Worktree Review child contract"));
    }

    #[test]
    fn versioned_contract_distinguishes_compatible_source_from_legacy_source() {
        let directory = tempfile::tempdir().expect("directory");
        let current = directory.path().join("compatible");
        let legacy = directory.path().join("legacy");
        std::fs::create_dir_all(current.join("src-tauri")).expect("current");
        std::fs::create_dir_all(&legacy).expect("legacy");
        std::fs::write(
            current.join("src-tauri/worktree-review-contract.json"),
            r#"{"version":1,"readiness":"owned-window-and-rendered-application","provenance":"worktree-build-details-v1"}"#,
        )
        .expect("marker");
        let text = format!(
            "worktree {}\nHEAD 0123456789abcdef\nbranch refs/heads/codex/current\n\nworktree {}\nHEAD abcdef0123456789\nbranch refs/heads/codex/legacy\n",
            current.display(),
            legacy.display()
        );
        let catalog = ReviewWorktreeCatalog::from_porcelain(
            &text,
            &current.canonicalize().expect("canonical current"),
        )
        .expect("catalog");
        let compatible = catalog
            .options
            .iter()
            .find(|option| option.label.contains("launcher source"))
            .expect("compatible");
        let legacy = catalog
            .options
            .iter()
            .find(|option| !option.label.contains("launcher source"))
            .expect("legacy");
        assert_eq!(compatible.compatibility, "compatible");
        assert!(catalog.ensure_compatible(&compatible.source_ref).is_ok());
        let error = catalog
            .ensure_compatible(&legacy.source_ref)
            .expect_err("legacy rejected");
        assert!(error.contains("waiting for a window cannot repair"));
    }
}
