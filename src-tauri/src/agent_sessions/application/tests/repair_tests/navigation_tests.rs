use super::*;
use crate::agent_sessions::organization::{SessionFolderTarget, SessionPlacement};
use crate::repository_catalog::RepositoryCatalog;
use crate::repository_context::PathIdentity;
use crate::session_navigation::application::{SessionNavigationService, StartSessionRequest};

#[test]
fn folder_creation_uses_main_tree_and_pinned_profile_without_workflow_ownership() {
    let fixture = Fixture::new();
    let main = fixture.folder.path().join("main-tree");
    let linked = fixture.folder.path().join("workflow-tree");
    std::fs::create_dir(&main).unwrap();
    let git = |arguments: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(&main)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-b", "main"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "--allow-empty",
        "-m",
        "Fixture",
    ]);
    git(&[
        "worktree",
        "add",
        "-b",
        "workflow",
        linked.to_str().unwrap(),
    ]);
    let database =
        crate::product_database::open(&fixture.folder.path().join("repair.sqlite")).unwrap();
    let catalog = Arc::new(RepositoryCatalog::new(database));
    let registered = catalog.register_directory(linked.clone()).unwrap();
    let base = fixture.instance(None);
    let mut target = fixture.target();
    target.repository.id = registered.repository_id.clone();
    target.worktree.path = linked.to_string_lossy().into_owned();
    let instance = fixture
        .execution
        .instances
        .create("Feature build".into(), base.recipe, target)
        .unwrap();
    let navigation = SessionNavigationService::new(
        catalog,
        fixture.execution.instances.clone(),
        fixture.repository.clone(),
        fixture.sessions.clone(),
    );

    for folder_target in [
        SessionFolderTarget::Repository {
            repository_id: registered.repository_id,
        },
        SessionFolderTarget::WorkflowInstance {
            instance_id: instance.id.clone(),
        },
    ] {
        let result = navigation
            .start_session(StartSessionRequest {
                submitted_text: "Ordinary notes".into(),
                title: Some("Project notes".into()),
                working_directory: Some(linked.to_string_lossy().into_owned()),
                model: None,
                reasoning_mode: None,
                sandbox_mode: None,
                folder_target: Some(folder_target.clone()),
            })
            .unwrap();
        let session_id = result.acknowledgement.session_id;
        let session = fixture
            .repository
            .get_session(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            PathIdentity::of(std::path::Path::new(
                session.working_directory.as_ref().unwrap()
            )),
            PathIdentity::of(&main)
        );
        assert!(session.session_profile.is_some());
        assert!(fixture
            .repository
            .load_address(&session_id)
            .unwrap()
            .is_none());
        let data = navigation.load().unwrap();
        assert!(data
            .owners
            .iter()
            .all(|owner| owner.session_id != session_id));
        let folder: SessionPlacement = folder_target.into();
        assert_eq!(
            data.organization
                .iter()
                .find(|record| record.session_id == session_id)
                .unwrap()
                .placement,
            folder
        );
        let flow = data
            .instances
            .iter()
            .find(|flow| flow.id == instance.id)
            .unwrap();
        assert_eq!(flow.name, "Feature build");
        assert_eq!(
            flow.nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        navigation.pin_session(session_id.clone(), true).unwrap();
        navigation
            .move_session(session_id.clone(), SessionPlacement::Unfiled)
            .unwrap();
        assert_eq!(
            fixture
                .repository
                .get_session(&session_id)
                .unwrap()
                .unwrap(),
            session
        );
    }
}
