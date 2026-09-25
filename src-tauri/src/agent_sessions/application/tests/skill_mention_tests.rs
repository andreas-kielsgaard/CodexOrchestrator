use super::*;
use crate::agent_sessions::ports::RuntimeSkillInput;
use orchid_engine::contracts::{ProviderConfigurationRef, ProviderSkill, ProviderSkillCatalogue};

struct NativeSkills {
    catalogue: ProviderSkillCatalogue,
    reads: AtomicU64,
}

impl ProviderConfigurationSource for NativeSkills {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        Ok(test_selected_runtime_profile())
    }
    fn native_skills_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<ProviderSkillCatalogue, ProviderConfigurationSourceError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.catalogue.clone())
    }
}

fn skill_file(root: &std::path::Path, name: &str) -> String {
    let path = root.join(name).join("SKILL.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, name).unwrap();
    path.canonicalize().unwrap().to_string_lossy().into_owned()
}

fn pinned_input(path: &str, name: &str) -> RuntimeSkillInput {
    RuntimeSkillInput {
        id: path.into(),
        name: name.into(),
        path: path.into(),
        content_sha256: "pinned".into(),
        description: String::new(),
    }
}

fn application_with(catalogue: ProviderSkillCatalogue) -> (AgentSessionApplication, Arc<NativeSkills>) {
    let source = Arc::new(NativeSkills {
        catalogue,
        reads: AtomicU64::new(0),
    });
    let application = Harness::new(RuntimeBehavior::CompleteWithBinding)
        .application
        .with_profile_source(source.clone());
    (application, source)
}

#[test]
fn pinned_sessions_invoke_only_their_pinned_skills_whatever_the_source() {
    let directory = tempfile::tempdir().unwrap();
    let product = skill_file(directory.path(), "review");
    let native = skill_file(directory.path(), "native");
    let (application, source) = application_with(ProviderSkillCatalogue {
        skills: vec![ProviderSkill {
            name: "native".into(),
            description: String::new(),
            path: native,
            scope: "user".into(),
            enabled: true,
        }],
        limitations: Vec::new(),
    });
    let mut extension = RuntimeLaunchExtension {
        skill_inputs: vec![pinned_input(&product, "review")],
        ..Default::default()
    };
    application.apply_skill_mentions(
        &ProviderConfigurationRef::new("codex", "selected"),
        None,
        "Use $review, then $native, but not $reviewer.",
        true,
        &mut extension,
    );
    assert_eq!(extension.invoked_skill_ids, [product]);
    assert_eq!(extension.skill_inputs.len(), 1);
    assert_eq!(source.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn unprofiled_sessions_pin_and_invoke_a_mentioned_native_skill() {
    let directory = tempfile::tempdir().unwrap();
    let native = skill_file(directory.path(), "native");
    let duplicate = |path: String| ProviderSkill {
        name: "twice".into(),
        description: String::new(),
        path,
        scope: "repo".into(),
        enabled: true,
    };
    let (application, _) = application_with(ProviderSkillCatalogue {
        skills: vec![
            ProviderSkill {
                name: "native".into(),
                description: "Native".into(),
                path: native.clone(),
                scope: "user".into(),
                enabled: true,
            },
            duplicate(skill_file(directory.path(), "one")),
            duplicate(skill_file(directory.path(), "two")),
        ],
        limitations: Vec::new(),
    });
    let mut extension = RuntimeLaunchExtension::default();
    application.apply_skill_mentions(
        &ProviderConfigurationRef::new("codex", "selected"),
        None,
        "Apply $native and the ambiguous $twice.",
        false,
        &mut extension,
    );
    assert_eq!(extension.skill_inputs.len(), 1);
    assert_eq!(extension.skill_inputs[0].path, native);
    assert_eq!(extension.invoked_skill_ids, [native]);
}
