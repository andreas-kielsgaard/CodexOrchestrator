use crate::worktree_runtime::{
    TestInstanceError, TestInstanceErrorKind, TestSourceRef, TestSourceResolver,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewWorktreeOption {
    pub(crate) source_ref: String,
    pub(crate) label: String,
    pub(crate) branch: Option<String>,
    pub(crate) detached: bool,
    pub(crate) is_main: bool,
    pub(crate) is_current: bool,
    pub(crate) parent_source_ref: Option<String>,
    pub(crate) lineage_ambiguous: bool,
    pub(crate) relationship: String,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
    pub(crate) fork_revision: String,
    pub(crate) revision: String,
    pub(crate) compatibility: String,
    pub(crate) compatibility_message: String,
    pub(crate) attached: bool,
    pub(crate) ref_kind: String,
    pub(crate) merged_directly: bool,
    pub(crate) equivalent_patches: usize,
    pub(crate) comparison_branch: String,
    object_id: String,
}

#[derive(Clone, Debug)]
struct DurableReviewRef {
    option: ReviewWorktreeOption,
    full_ref: String,
}

pub(crate) struct ReviewWorktreeCatalog {
    options: Vec<ReviewWorktreeOption>,
    paths: HashMap<String, PathBuf>,
    main_path: PathBuf,
    /// Discovery-time immutable baseline; later machine-main HEAD movement does not replace it.
    main_head: String,
    common_dir: Option<PathBuf>,
    git: Option<PathBuf>,
    comparison_branch: String,
    durable_refs: Vec<DurableReviewRef>,
    attached_paths: Mutex<HashMap<String, PathBuf>>,
}

pub(super) struct CatalogComparisonIdentity {
    pub(super) main_root: PathBuf,
    pub(super) selected_root: PathBuf,
    pub(super) baseline_object_id: String,
    pub(super) common_dir: PathBuf,
}

pub(super) struct CatalogSourceHistoryIdentity {
    pub(super) selected_root: PathBuf,
    pub(super) baseline_object_id: String,
    pub(super) selected_object_id: String,
    pub(super) branch: String,
    pub(super) source_label: String,
    pub(super) related_branch_tips: Vec<(String, String)>,
}

pub(super) struct CatalogSourceFreshness {
    pub(super) prepared_revision: String,
    pub(super) current_revision: String,
    pub(super) state: String,
    pub(super) outdated_by_commits: Option<usize>,
}

impl ReviewWorktreeCatalog {
    pub(crate) fn discover(current_source: &Path, git: &Path) -> Result<Self, String> {
        Self::discover_with_comparison(current_source, git, None)
    }

    pub(crate) fn discover_with_comparison(
        current_source: &Path,
        git: &Path,
        comparison_branch: Option<&str>,
    ) -> Result<Self, String> {
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
        let mut catalog = validate_catalog_identity(
            Self::from_porcelain(&text, &current_source)?,
            &current_source,
            |path| git_common_dir(path, git),
        )?;
        catalog.git = Some(git.to_path_buf());
        if let Some(branch) = comparison_branch {
            let object_id = git_text(
                &catalog.main_path,
                git,
                &["rev-parse", "--verify", &format!("{branch}^{{commit}}")],
            )?;
            catalog.main_head = object_id;
            catalog.comparison_branch = branch.to_owned();
        } else {
            catalog.comparison_branch = catalog
                .options
                .iter()
                .find(|option| option.is_main)
                .and_then(|option| option.branch.clone())
                .unwrap_or_else(|| "main".into());
        }
        catalog.populate_relationships(git)?;
        catalog.populate_durable_refs(git)?;
        Ok(catalog)
    }

    fn from_porcelain(text: &str, current_source: &Path) -> Result<Self, String> {
        let mut options = Vec::new();
        let mut paths = HashMap::new();
        let mut main_path = None;
        let mut main_head = None;
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
            let is_main = main_path.is_none();
            if is_main {
                main_path = Some(path.clone());
                main_head = head.clone();
            }
            let head = head.ok_or_else(|| "Git returned a worktree without HEAD".to_string())?;
            let digest = format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()));
            let source_ref = format!("review-source-{}", &digest[..20]);
            let detached = branch.is_none();
            let base = branch
                .clone()
                .unwrap_or_else(|| format!("Detached {}", &head[..head.len().min(8)]));
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
                branch,
                detached,
                is_main,
                is_current: path == current_source,
                parent_source_ref: None,
                lineage_ambiguous: false,
                relationship: "related".into(),
                ahead: 0,
                behind: 0,
                fork_revision: head[..head.len().min(12)].to_owned(),
                revision: head[..head.len().min(12)].to_owned(),
                compatibility,
                compatibility_message,
                attached: true,
                ref_kind: if detached { "detached" } else { "branch" }.into(),
                merged_directly: false,
                equivalent_patches: 0,
                comparison_branch: "main".into(),
                object_id: head,
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
            main_head: main_head
                .ok_or_else(|| "No machine-main Git object was discovered".to_string())?,
            common_dir: None,
            git: None,
            comparison_branch: "main".into(),
            durable_refs: Vec::new(),
            attached_paths: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn options(&self) -> &[ReviewWorktreeOption] {
        &self.options
    }

    pub(crate) fn live_options(&self) -> Result<Vec<ReviewWorktreeOption>, String> {
        let snapshot = self.live_snapshot()?;
        let mut options = snapshot.options;
        let attached = self
            .attached_paths
            .lock()
            .map(|paths| paths.clone())
            .unwrap_or_default();
        for durable in &self.durable_refs {
            let mut option = durable.option.clone();
            let path = attached
                .get(&option.source_ref)
                .cloned()
                .or_else(|| self.registered_attachment(&option.source_ref));
            if let Some(path) = path {
                option.attached = true;
                let (compatibility, message) = compatibility(&path);
                option.compatibility = compatibility;
                option.compatibility_message = message;
            }
            options.push(option);
        }
        options.sort_by(|left, right| {
            right
                .is_main
                .cmp(&left.is_main)
                .then_with(|| left.branch.cmp(&right.branch))
                .then_with(|| left.source_ref.cmp(&right.source_ref))
        });
        Ok(options)
    }

    fn live_snapshot(&self) -> Result<Self, String> {
        let Some(git) = self.git.as_deref() else {
            return Ok(Self {
                options: self.options.clone(),
                paths: self.paths.clone(),
                main_path: self.main_path.clone(),
                main_head: self.main_head.clone(),
                common_dir: self.common_dir.clone(),
                git: None,
                comparison_branch: self.comparison_branch.clone(),
                durable_refs: self.durable_refs.clone(),
                attached_paths: Mutex::new(
                    self.attached_paths
                        .lock()
                        .map(|paths| paths.clone())
                        .unwrap_or_default(),
                ),
            });
        };
        let mut options = Vec::new();
        let mut paths = HashMap::new();
        for original in &self.options {
            let Some(path) = self.paths.get(&original.source_ref) else {
                continue;
            };
            if path.canonicalize().ok().as_ref() != Some(path) {
                continue;
            }
            let object_id = git_text(path, git, &["rev-parse", "--verify", "HEAD^{commit}"])?;
            let branch =
                git_optional_text(path, git, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
            let detached = branch.is_none();
            let base = branch
                .clone()
                .unwrap_or_else(|| format!("Detached {}", abbreviated(&object_id, 8)));
            let folder = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("worktree");
            let label = if original.is_current {
                format!("{base} - launcher source")
            } else {
                format!("{base} - {folder}")
            };
            let (compatibility, compatibility_message) = compatibility(path);
            options.push(ReviewWorktreeOption {
                source_ref: original.source_ref.clone(),
                label,
                branch,
                detached,
                is_main: original.is_main,
                is_current: original.is_current,
                parent_source_ref: None,
                lineage_ambiguous: false,
                relationship: "related".into(),
                ahead: 0,
                behind: 0,
                fork_revision: abbreviated(&object_id, 12),
                revision: abbreviated(&object_id, 12),
                compatibility,
                compatibility_message,
                attached: true,
                ref_kind: if detached { "detached" } else { "branch" }.into(),
                merged_directly: false,
                equivalent_patches: 0,
                comparison_branch: original.comparison_branch.clone(),
                object_id,
            });
            paths.insert(original.source_ref.clone(), path.clone());
        }
        let mut snapshot = Self {
            options,
            paths,
            main_path: self.main_path.clone(),
            main_head: self.main_head.clone(),
            common_dir: self.common_dir.clone(),
            git: self.git.clone(),
            comparison_branch: self.comparison_branch.clone(),
            durable_refs: self.durable_refs.clone(),
            attached_paths: Mutex::new(
                self.attached_paths
                    .lock()
                    .map(|paths| paths.clone())
                    .unwrap_or_default(),
            ),
        };
        snapshot.populate_relationships(git)?;
        Ok(snapshot)
    }

    fn populate_relationships(&mut self, git: &Path) -> Result<(), String> {
        let comparison_branch = self.comparison_branch.clone();
        let main_ref = self
            .options
            .iter()
            .find(|option| option.is_main)
            .map(|option| option.source_ref.clone())
            .ok_or_else(|| "No machine-main worktree was discovered".to_string())?;
        let branch_tips = self
            .options
            .iter()
            .filter(|option| !option.detached)
            .map(|option| {
                (
                    option.source_ref.clone(),
                    option.label.clone(),
                    option.object_id.clone(),
                    option.is_main,
                )
            })
            .collect::<Vec<_>>();

        for option in &mut self.options {
            let path = self
                .paths
                .get(&option.source_ref)
                .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
            let range = format!("{}...{}", self.main_head, option.object_id);
            let counts = git_text(path, git, &["rev-list", "--left-right", "--count", &range])?;
            let mut counts = counts.split_whitespace();
            option.behind = counts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            option.ahead = counts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let merge_base = git_text(
                path,
                git,
                &["merge-base", &self.main_head, &option.object_id],
            )
            .ok();
            option.relationship = if merge_base.is_some() {
                "related".into()
            } else {
                "unrelated".into()
            };
            option.merged_directly = git_success(
                path,
                git,
                &[
                    "merge-base",
                    "--is-ancestor",
                    &option.object_id,
                    &self.main_head,
                ],
            );
            option.equivalent_patches =
                equivalent_patch_count(path, git, &self.main_head, &option.object_id);
            option.comparison_branch = comparison_branch.clone();
            option.fork_revision = merge_base
                .as_deref()
                .map(|value| abbreviated(value, 12))
                .unwrap_or_else(|| "No common ancestor".into());

            if option.is_main || option.detached || option.relationship == "unrelated" {
                continue;
            }
            let first_parent_commits = git_text(
                path,
                git,
                &["rev-list", "--first-parent", &option.object_id],
            )?
            .lines()
            .map(str::to_owned)
            .collect::<HashSet<_>>();
            let mut candidates = branch_tips
                .iter()
                .filter(|(source_ref, _, object_id, is_main)| {
                    source_ref != &option.source_ref
                        && !is_main
                        && object_id != &option.object_id
                        && first_parent_commits.contains(object_id)
                        && git_success(
                            path,
                            git,
                            &["merge-base", "--is-ancestor", object_id, &option.object_id],
                        )
                })
                .filter_map(|(source_ref, label, object_id, _)| {
                    let range = format!("{object_id}..{}", option.object_id);
                    git_text(path, git, &["rev-list", "--count", &range])
                        .ok()
                        .and_then(|distance| distance.parse::<usize>().ok())
                        .map(|distance| (distance, label, source_ref))
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| left.cmp(right));
            let nearest_distance = candidates.first().map(|candidate| candidate.0);
            option.lineage_ambiguous = nearest_distance.is_some_and(|distance| {
                candidates
                    .iter()
                    .filter(|candidate| candidate.0 == distance)
                    .count()
                    > 1
            });
            option.parent_source_ref = if option.lineage_ambiguous {
                Some(main_ref.clone())
            } else {
                candidates
                    .first()
                    .map(|(_, _, source_ref)| (*source_ref).clone())
                    .or_else(|| Some(main_ref.clone()))
            };
        }
        Ok(())
    }

    fn populate_durable_refs(&mut self, git: &Path) -> Result<(), String> {
        let output = Command::new(git)
            .arg("-C")
            .arg(&self.main_path)
            .args([
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00%(symref)",
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .map_err(|error| format!("discover durable Git references: {error}"))?;
        if !output.status.success() {
            return Err("Git durable-reference discovery failed".into());
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|_| "Git returned non-UTF-8 durable references".to_string())?;
        let existing = self
            .options
            .iter()
            .filter_map(|option| option.branch.clone())
            .collect::<HashSet<_>>();
        let comparison_branch = self.comparison_branch.clone();
        let mut seen = HashSet::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let mut fields = line.split('\0');
            let Some(full_ref) = fields.next() else {
                continue;
            };
            let Some(object_id) = fields.next() else {
                continue;
            };
            let symref = fields.next().unwrap_or_default();
            if !symref.is_empty() || object_id.is_empty() {
                continue;
            }
            let Some((display, ref_kind)) = display_ref(full_ref) else {
                continue;
            };
            if existing.contains(&display) || !seen.insert(display.clone()) {
                continue;
            }
            let selected_object = match git_text(
                &self.main_path,
                git,
                &["rev-parse", "--verify", &format!("{full_ref}^{{commit}}")],
            ) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let merge_base = git_text(
                &self.main_path,
                git,
                &["merge-base", &self.main_head, &selected_object],
            )
            .ok();
            let range = format!("{}...{}", self.main_head, selected_object);
            let (behind, ahead) = git_text(
                &self.main_path,
                git,
                &["rev-list", "--left-right", "--count", &range],
            )
            .ok()
            .and_then(|counts| {
                let mut counts = counts.split_whitespace();
                Some((counts.next()?.parse().ok()?, counts.next()?.parse().ok()?))
            })
            .unwrap_or((0, 0));
            let digest = format!("{:x}", Sha256::digest(full_ref.as_bytes()));
            let source_ref = format!("review-ref-{}", &digest[..20]);
            let merged_directly = git_success(
                &self.main_path,
                git,
                &[
                    "merge-base",
                    "--is-ancestor",
                    &selected_object,
                    &self.main_head,
                ],
            );
            let option = ReviewWorktreeOption {
                source_ref,
                label: display.clone(),
                branch: Some(display),
                detached: false,
                is_main: false,
                is_current: false,
                parent_source_ref: None,
                lineage_ambiguous: false,
                relationship: if merge_base.is_some() {
                    "related".into()
                } else {
                    "unrelated".into()
                },
                ahead,
                behind,
                fork_revision: merge_base
                    .as_deref()
                    .map(|value| abbreviated(value, 12))
                    .unwrap_or_else(|| "No common ancestor".into()),
                revision: abbreviated(&selected_object, 12),
                compatibility: "unavailable".into(),
                compatibility_message: "Attach a review worktree before preparing a build.".into(),
                attached: false,
                ref_kind: ref_kind.into(),
                merged_directly,
                equivalent_patches: equivalent_patch_count(
                    &self.main_path,
                    git,
                    &self.main_head,
                    &selected_object,
                ),
                comparison_branch: comparison_branch.clone(),
                object_id: selected_object,
            };
            self.durable_refs.push(DurableReviewRef {
                option,
                full_ref: full_ref.into(),
            });
        }
        Ok(())
    }

    pub(crate) fn label(&self, source_ref: &str) -> Option<String> {
        self.live_options()
            .ok()?
            .into_iter()
            .find(|option| option.source_ref == source_ref)
            .map(|option| option.label)
    }

    pub(crate) fn ensure_compatible(&self, source_ref: &str) -> Result<(), String> {
        let options = self.live_options()?;
        let option = options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        if !option.attached {
            return Err("Attach a review worktree before preparing a build.".into());
        }
        if option.compatibility == "compatible" {
            Ok(())
        } else {
            Err(option.compatibility_message.clone())
        }
    }

    pub(crate) fn compatibility(&self, source_ref: &str) -> (String, String) {
        self.live_options()
            .unwrap_or_default()
            .into_iter()
            .find(|option| option.source_ref == source_ref)
            .map(|option| (option.compatibility, option.compatibility_message))
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
            .path_for(source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        Ok(super::worktree_build::WorktreeScope {
            name,
            selected,
            main: self.main_path.clone(),
        })
    }

    pub(super) fn comparison_identity(
        &self,
        source_ref: &str,
    ) -> Result<CatalogComparisonIdentity, String> {
        let selected = self
            .path_for(source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        Ok(CatalogComparisonIdentity {
            main_root: self.main_path.clone(),
            selected_root: selected,
            baseline_object_id: self.main_head.clone(),
            common_dir: self
                .common_dir
                .clone()
                .ok_or_else(|| "The catalog Git repository identity is unavailable.".to_string())?,
        })
    }

    pub(super) fn source_history_identity(
        &self,
        source_ref: &str,
    ) -> Result<CatalogSourceHistoryIdentity, String> {
        let snapshot = self.live_snapshot()?;
        let options = self.live_options()?;
        let option = options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        let branch = option
            .branch
            .clone()
            .ok_or_else(|| "Commit history is available for named branches only.".to_string())?;
        if option.relationship != "related" {
            return Err(
                "Commit history requires a common ancestor with the machine-main branch."
                    .to_string(),
            );
        }
        let selected_root = self
            .path_for(source_ref)
            .unwrap_or_else(|| snapshot.main_path.clone());
        Ok(CatalogSourceHistoryIdentity {
            selected_root,
            baseline_object_id: snapshot.main_head.clone(),
            selected_object_id: option.object_id.clone(),
            branch,
            source_label: option.label.clone(),
            related_branch_tips: options
                .iter()
                .filter(|candidate| {
                    !candidate.is_main
                        && !candidate.detached
                        && candidate.source_ref != option.source_ref
                })
                .filter_map(|candidate| {
                    candidate
                        .branch
                        .clone()
                        .map(|branch| (branch, candidate.object_id.clone()))
                })
                .collect(),
        })
    }

    pub(super) fn source_freshness(
        &self,
        source_ref: &str,
        prepared_object_id: &str,
    ) -> Result<CatalogSourceFreshness, String> {
        let git = self
            .git
            .as_deref()
            .ok_or_else(|| "The catalog Git executable is unavailable.".to_string())?;
        let options = self.live_options()?;
        let option = options
            .iter()
            .find(|option| option.source_ref == source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        let path = self
            .path_for(source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        let prepared = git_text(
            &path,
            git,
            &[
                "rev-parse",
                "--verify",
                &format!("{prepared_object_id}^{{commit}}"),
            ],
        )?;
        if prepared == option.object_id {
            return Ok(CatalogSourceFreshness {
                prepared_revision: abbreviated(&prepared, 12),
                current_revision: abbreviated(&option.object_id, 12),
                state: "current".into(),
                outdated_by_commits: Some(0),
            });
        }
        if git_success(
            &path,
            git,
            &["merge-base", "--is-ancestor", &prepared, &option.object_id],
        ) {
            let count = git_text(
                &path,
                git,
                &[
                    "rev-list",
                    "--count",
                    &format!("{prepared}..{}", option.object_id),
                ],
            )?
            .parse::<usize>()
            .map_err(|_| "Git returned an invalid retained-build distance.".to_string())?;
            return Ok(CatalogSourceFreshness {
                prepared_revision: abbreviated(&prepared, 12),
                current_revision: abbreviated(&option.object_id, 12),
                state: "outdated".into(),
                outdated_by_commits: Some(count),
            });
        }
        Ok(CatalogSourceFreshness {
            prepared_revision: abbreviated(&prepared, 12),
            current_revision: abbreviated(&option.object_id, 12),
            state: "changed".into(),
            outdated_by_commits: None,
        })
    }

    pub(crate) fn attach_review_worktree(
        &self,
        source_ref: &str,
        attachments_root: &Path,
    ) -> Result<ReviewWorktreeOption, String> {
        let durable = self
            .durable_refs
            .iter()
            .find(|candidate| candidate.option.source_ref == source_ref)
            .ok_or_else(|| "The selected durable Git reference is unavailable.".to_string())?;
        if let Some(path) = self.path_for(source_ref) {
            let mut option = durable.option.clone();
            option.attached = true;
            let (compatibility, message) = compatibility(&path);
            option.compatibility = compatibility;
            option.compatibility_message = message;
            return Ok(option);
        }
        fs::create_dir_all(attachments_root)
            .map_err(|error| format!("create review worktree root: {error}"))?;
        let target = attachments_root.join(source_ref);
        if target.exists() {
            return Err("The review worktree path already exists but is not registered.".into());
        }
        let git = self
            .git
            .as_deref()
            .ok_or_else(|| "The catalog Git executable is unavailable.".to_string())?;
        let output = Command::new(git)
            .arg("-C")
            .arg(&self.main_path)
            .args(["worktree", "add", "--detach"])
            .arg(&target)
            .arg(&durable.full_ref)
            .output()
            .map_err(|error| format!("attach review worktree: {error}"))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(if detail.is_empty() {
                "Git could not attach the review worktree.".into()
            } else {
                format!("Git could not attach the review worktree: {detail}")
            });
        }
        let path = target
            .canonicalize()
            .map_err(|error| format!("resolve attached review worktree: {error}"))?;
        self.attached_paths
            .lock()
            .map_err(|_| "Review worktree attachment state is unavailable.".to_string())?
            .insert(source_ref.to_owned(), path.clone());
        let mut option = durable.option.clone();
        option.attached = true;
        let (compatibility, message) = compatibility(&path);
        option.compatibility = compatibility;
        option.compatibility_message = message;
        Ok(option)
    }

    fn path_for(&self, source_ref: &str) -> Option<PathBuf> {
        self.paths
            .get(source_ref)
            .cloned()
            .or_else(|| {
                self.attached_paths
                    .lock()
                    .ok()
                    .and_then(|paths| paths.get(source_ref).cloned())
            })
            .or_else(|| self.registered_attachment(source_ref))
    }

    fn registered_attachment(&self, source_ref: &str) -> Option<PathBuf> {
        self.paths.values().find_map(|path| {
            (path.file_name().and_then(|name| name.to_str()) == Some(source_ref))
                .then(|| path.clone())
        })
    }
}

fn abbreviated(value: &str, length: usize) -> String {
    value.chars().take(length).collect()
}

fn display_ref(full_ref: &str) -> Option<(String, &'static str)> {
    if let Some(branch) = full_ref.strip_prefix("refs/heads/") {
        return Some((branch.into(), "branch"));
    }
    if let Some(branch) = full_ref.strip_prefix("refs/remotes/") {
        if branch.ends_with("/HEAD") {
            return None;
        }
        return Some((branch.into(), "remote_branch"));
    }
    let tag = full_ref.strip_prefix("refs/tags/")?;
    if let Some(archived) = tag.strip_prefix("archive/") {
        let (_, original) = archived.split_once('/')?;
        return Some((original.into(), "archive"));
    }
    Some((tag.into(), "tag"))
}

fn equivalent_patch_count(path: &Path, git: &Path, baseline: &str, selected: &str) -> usize {
    git_text(path, git, &["cherry", baseline, selected])
        .map(|output| output.lines().filter(|line| line.starts_with('-')).count())
        .unwrap_or(0)
}

fn git_text(path: &Path, git: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(git)
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| format!("inspect Git worktree relationships: {error}"))?;
    if !output.status.success() {
        return Err("Git could not inspect worktree relationships".into());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| "Git returned non-UTF-8 worktree relationships".to_string())
}

fn git_optional_text(path: &Path, git: &Path, args: &[&str]) -> Result<Option<String>, String> {
    let output = Command::new(git)
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| format!("inspect Git worktree identity: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    String::from_utf8(output.stdout)
        .map(|value| Some(value.trim().to_owned()))
        .map_err(|_| "Git returned non-UTF-8 worktree identity".to_string())
}

fn git_success(path: &Path, git: &Path, args: &[&str]) -> bool {
    Command::new(git)
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn validate_catalog_identity(
    mut catalog: ReviewWorktreeCatalog,
    current_source: &Path,
    mut common_dir_for: impl FnMut(&Path) -> Result<PathBuf, String>,
) -> Result<ReviewWorktreeCatalog, String> {
    if !catalog.paths.values().any(|path| path == current_source) {
        return Err("The launcher source is missing from Git worktree discovery".into());
    }
    let common_dir = common_dir_for(&catalog.main_path)?;
    if common_dir_for(current_source)? != common_dir {
        return Err("The launcher source does not belong to the catalog repository".into());
    }
    let usable = catalog
        .paths
        .iter()
        .filter_map(|(source_ref, path)| {
            (path == &catalog.main_path
                || path == current_source
                || common_dir_for(path).is_ok_and(|candidate| candidate == common_dir))
            .then(|| source_ref.clone())
        })
        .collect::<HashSet<_>>();
    catalog
        .paths
        .retain(|source_ref, _| usable.contains(source_ref));
    catalog
        .options
        .retain(|option| usable.contains(&option.source_ref));
    catalog.common_dir = Some(common_dir);
    Ok(catalog)
}

fn git_common_dir(root: &Path, git: &Path) -> Result<PathBuf, String> {
    let output = Command::new(git)
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|error| format!("resolve Git common directory: {error}"))?;
    if !output.status.success() {
        return Err("Git common-directory discovery failed".into());
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|_| "Git common-directory output was not UTF-8".to_string())?;
    let path = PathBuf::from(value.trim());
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    path.canonicalize()
        .map_err(|error| format!("resolve canonical Git common directory: {error}"))
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
        self.path_for(source.as_str()).ok_or_else(|| {
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

    #[test]
    fn discovered_full_baseline_does_not_follow_later_machine_main_head() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("source.txt"), "baseline\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "baseline"]);
        let baseline = git_output(&main, &["rev-parse", "HEAD"]);
        git(&main, &["branch", "feature"]);
        git(
            &main,
            &["worktree", "add", selected.to_str().unwrap(), "feature"],
        );
        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).unwrap();
        let selected_ref = catalog
            .options()
            .iter()
            .find(|option| !option.label.contains("launcher source"))
            .unwrap()
            .source_ref
            .clone();

        fs::write(main.join("later.txt"), "later\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "later"]);
        let current = git_output(&main, &["rev-parse", "HEAD"]);
        assert_ne!(current, baseline);
        let live_main = catalog
            .live_options()
            .unwrap()
            .into_iter()
            .find(|option| option.is_main)
            .expect("live main");
        assert_eq!(live_main.revision, abbreviated(&current, 12));

        assert_eq!(
            catalog
                .comparison_identity(&selected_ref)
                .unwrap()
                .baseline_object_id,
            baseline
        );
    }

    #[test]
    fn retained_source_freshness_distinguishes_current_outdated_and_changed_history() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("source.txt"), "baseline\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "baseline"]);
        let baseline = git_output(&main, &["rev-parse", "HEAD"]);
        git(&main, &["branch", "feature"]);
        git(
            &main,
            &["worktree", "add", selected.to_str().unwrap(), "feature"],
        );
        fs::write(selected.join("prepared.txt"), "prepared\n").unwrap();
        git(&selected, &["add", "."]);
        git(&selected, &["commit", "-m", "prepared"]);
        let prepared = git_output(&selected, &["rev-parse", "HEAD"]);
        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).unwrap();
        let selected_ref = catalog
            .options()
            .iter()
            .find(|option| option.branch.as_deref() == Some("feature"))
            .unwrap()
            .source_ref
            .clone();

        let current = catalog
            .source_freshness(&selected_ref, &prepared)
            .expect("current freshness");
        assert_eq!(current.state, "current");
        assert_eq!(current.outdated_by_commits, Some(0));

        for index in 1..=2 {
            fs::write(
                selected.join(format!("later-{index}.txt")),
                format!("later {index}\n"),
            )
            .unwrap();
            git(&selected, &["add", "."]);
            git(&selected, &["commit", "-m", &format!("later {index}")]);
        }
        let advanced = git_output(&selected, &["rev-parse", "HEAD"]);
        let outdated = catalog
            .source_freshness(&selected_ref, &prepared)
            .expect("outdated freshness");
        assert_eq!(outdated.state, "outdated");
        assert_eq!(outdated.outdated_by_commits, Some(2));
        assert_eq!(outdated.prepared_revision, abbreviated(&prepared, 12));
        assert_eq!(outdated.current_revision, abbreviated(&advanced, 12));

        git(&selected, &["reset", "--hard", &baseline]);
        fs::write(selected.join("replacement.txt"), "replacement\n").unwrap();
        git(&selected, &["add", "."]);
        git(&selected, &["commit", "-m", "replacement history"]);
        let changed = catalog
            .source_freshness(&selected_ref, &prepared)
            .expect("changed freshness");
        assert_eq!(changed.state, "changed");
        assert_eq!(changed.outdated_by_commits, None);
    }

    #[test]
    fn discovery_keeps_an_unusable_registered_branch_as_an_unattached_reference() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        let unavailable = directory.path().join("unavailable");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("source.txt"), "baseline\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "baseline"]);
        git(&main, &["branch", "selected"]);
        git(&main, &["branch", "unavailable"]);
        git(
            &main,
            &["worktree", "add", selected.to_str().unwrap(), "selected"],
        );
        git(
            &main,
            &[
                "worktree",
                "add",
                unavailable.to_str().unwrap(),
                "unavailable",
            ],
        );
        fs::write(unavailable.join(".git"), "gitdir: missing\n").unwrap();

        let review = crate::worktree_review::compose(&main, &directory.path().join("runtime"))
            .expect("unrelated unavailable worktree does not prevent review startup");
        let sources = review.sources();
        assert_eq!(sources.len(), 3);
        assert!(sources
            .iter()
            .any(|option| option.label.contains("launcher source")));
        assert!(sources
            .iter()
            .any(|option| option.label.contains("selected")));
        let unavailable = sources
            .iter()
            .find(|option| option.label.contains("unavailable"))
            .expect("durable branch remains selectable");
        assert!(!unavailable.attached);
    }

    #[test]
    fn archived_branch_can_be_attached_as_an_exact_detached_review_worktree() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::create_dir_all(main.join("src-tauri")).unwrap();
        fs::write(
            main.join("src-tauri/worktree-review-contract.json"),
            r#"{"version":1,"readiness":"owned-window-and-rendered-application","provenance":"worktree-build-details-v1"}"#,
        )
        .unwrap();
        fs::write(main.join("source.txt"), "baseline\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "baseline"]);
        git(&main, &["switch", "-c", "codex/explore-harness-inspector"]);
        fs::write(main.join("feature.txt"), "archived work\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "archived feature"]);
        let archived_head = git_output(&main, &["rev-parse", "HEAD"]);
        git(
            &main,
            &["tag", "archive/2026-08-08/codex/explore-harness-inspector"],
        );
        git(&main, &["switch", "master"]);
        git(&main, &["branch", "-D", "codex/explore-harness-inspector"]);

        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).unwrap();
        let archived = catalog
            .live_options()
            .unwrap()
            .into_iter()
            .find(|option| option.ref_kind == "archive")
            .expect("archive reference");
        assert_eq!(
            archived.branch.as_deref(),
            Some("codex/explore-harness-inspector")
        );
        assert!(!archived.attached);

        let attached = catalog
            .attach_review_worktree(&archived.source_ref, &directory.path().join("attachments"))
            .unwrap();
        assert!(attached.attached);
        let attached_path = catalog
            .path_for(&archived.source_ref)
            .expect("attached path");
        assert_eq!(
            git_output(&attached_path, &["rev-parse", "HEAD"]),
            archived_head
        );
        let symbolic = Command::new("git")
            .arg("-C")
            .arg(&attached_path)
            .args(["symbolic-ref", "--quiet", "HEAD"])
            .status()
            .unwrap();
        assert!(!symbolic.success(), "review worktree must remain detached");
    }

    #[test]
    fn equally_near_registered_branch_tips_do_not_invent_one_parent() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let parent_one = directory.path().join("parent-one");
        let parent_two = directory.path().join("parent-two");
        let child = directory.path().join("child");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("base.txt"), "base\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "base"]);
        git(
            &main,
            &[
                "worktree",
                "add",
                "-b",
                "codex/parent-one",
                parent_one.to_str().unwrap(),
            ],
        );
        fs::write(parent_one.join("parent.txt"), "parent\n").unwrap();
        git(&parent_one, &["add", "."]);
        git(&parent_one, &["commit", "-m", "parent"]);
        git(&main, &["branch", "codex/parent-two", "codex/parent-one"]);
        git(
            &main,
            &[
                "worktree",
                "add",
                parent_two.to_str().unwrap(),
                "codex/parent-two",
            ],
        );
        git(
            &main,
            &[
                "worktree",
                "add",
                "-b",
                "codex/child",
                child.to_str().unwrap(),
                "codex/parent-one",
            ],
        );
        fs::write(child.join("child.txt"), "child\n").unwrap();
        git(&child, &["add", "."]);
        git(&child, &["commit", "-m", "child"]);

        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).unwrap();
        let main_ref = catalog
            .options()
            .iter()
            .find(|option| option.is_main)
            .unwrap()
            .source_ref
            .clone();
        let child = catalog
            .options()
            .iter()
            .find(|option| option.branch.as_deref() == Some("codex/child"))
            .unwrap();
        assert!(child.lineage_ambiguous);
        assert_eq!(child.parent_source_ref.as_deref(), Some(main_ref.as_str()));
    }

    #[test]
    fn unrelated_registered_history_is_not_nested_under_main() {
        let directory = tempfile::tempdir().expect("directory");
        let main = directory.path().join("main");
        let orphan = directory.path().join("orphan");
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("base.txt"), "base\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "base"]);
        git(
            &main,
            &[
                "worktree",
                "add",
                "--orphan",
                "-b",
                "codex/orphan",
                orphan.to_str().unwrap(),
            ],
        );
        fs::write(orphan.join("orphan.txt"), "orphan\n").unwrap();
        git(&orphan, &["add", "."]);
        git(&orphan, &["commit", "-m", "orphan"]);

        let catalog = ReviewWorktreeCatalog::discover(&main, Path::new("git")).unwrap();
        let orphan = catalog
            .options()
            .iter()
            .find(|option| option.branch.as_deref() == Some("codex/orphan"))
            .unwrap();
        assert_eq!(orphan.relationship, "unrelated");
        assert!(orphan.parent_source_ref.is_none());
        assert_eq!(orphan.fork_revision, "No common ancestor");
        assert!(catalog.source_history_identity(&orphan.source_ref).is_err());
    }

    #[test]
    fn catalog_rejects_source_mismatch_and_excludes_foreign_unselected_entries() {
        let directory = tempfile::tempdir().expect("directory");
        let current = directory.path().join("current");
        let foreign = directory.path().join("foreign");
        fs::create_dir_all(&current).expect("current");
        fs::create_dir_all(&foreign).expect("foreign");
        let current = current.canonicalize().expect("current canonical");
        let foreign = foreign.canonicalize().expect("foreign canonical");
        let text = format!(
            "worktree {}\nHEAD 0123456789abcdef\nbranch refs/heads/current\n\nworktree {}\nHEAD abcdef0123456789\nbranch refs/heads/foreign\n",
            current.display(),
            foreign.display()
        );
        let catalog = ReviewWorktreeCatalog::from_porcelain(&text, &current).expect("catalog");
        let foreign_ref = catalog
            .options()
            .iter()
            .find(|option| option.label.contains("foreign"))
            .expect("foreign option")
            .source_ref
            .clone();
        let common = directory.path().join("common");
        fs::create_dir_all(&common).expect("common");
        let filtered = validate_catalog_identity(catalog, &current, |path| {
            if path == current {
                Ok(common.clone())
            } else {
                Ok(directory.path().join("foreign-common"))
            }
        })
        .expect("foreign entry is excluded");
        assert!(filtered.label(&foreign_ref).is_none());
        assert!(filtered.scope(&foreign_ref, "review".into()).is_err());

        let launcher = directory.path().join("launcher");
        fs::create_dir_all(&launcher).expect("launcher");
        let launcher = launcher.canonicalize().expect("launcher canonical");
        let source_text = format!(
            "worktree {}\nHEAD 0123456789abcdef\nbranch refs/heads/main\n\nworktree {}\nHEAD abcdef0123456789\nbranch refs/heads/launcher\n",
            current.display(),
            launcher.display()
        );
        let source_mismatch =
            ReviewWorktreeCatalog::from_porcelain(&source_text, &launcher).expect("catalog");
        let result = validate_catalog_identity(source_mismatch, &launcher, |path| {
            Ok(if path == launcher {
                directory.path().join("mismatched-common")
            } else {
                common.clone()
            })
        });
        let error = match result {
            Ok(_) => panic!("source mismatch accepted"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "The launcher source does not belong to the catalog repository"
        );
    }

    fn git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
}
