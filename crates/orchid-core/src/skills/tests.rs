use super::*;
use crate::synchronization::definitions::DefinitionStore;
fn database() -> std::sync::Arc<crate::persistence::ActiveDatabase> {
    std::sync::Arc::new(
        crate::persistence::ActiveDatabase::from_connection(
            rusqlite::Connection::open_in_memory().unwrap(),
            crate::schema::initialize,
        )
        .unwrap(),
    )
}
#[test]
fn imported_packages_retain_supporting_files_and_refresh_only_changed_origins() {
    let source = tempfile::tempdir().unwrap();
    let owned = tempfile::tempdir().unwrap();
    std::fs::create_dir(source.path().join("scripts")).unwrap();
    std::fs::write(source.path().join("SKILL.md"), "original").unwrap();
    std::fs::write(source.path().join("scripts/run.py"), "print('hello')").unwrap();
    let catalogue = SkillCatalogue::new(database(), owned.path().into()).unwrap();
    let package = catalogue
        .import(
            &source.path().join("SKILL.md"),
            "review",
            "Review code",
            "native",
        )
        .unwrap();
    let path = catalogue.materialize(&package).unwrap();
    assert_eq!(
        std::fs::read_to_string(path.parent().unwrap().join("scripts/run.py")).unwrap(),
        "print('hello')"
    );
    let mut without_script = package.clone();
    without_script.files.remove("scripts/run.py");
    catalogue.materialize(&without_script).unwrap();
    assert!(!path.parent().unwrap().join("scripts/run.py").exists());
    let mut edited = package.clone();
    edited.files.get_mut("SKILL.md").unwrap().data_base64 =
        base64::engine::general_purpose::STANDARD.encode("edited Orchid copy");
    catalogue.store.save_skill(&edited).unwrap();
    SkillCatalogue::refresh_origins(&catalogue.store).unwrap();
    assert_eq!(catalogue.store.skills().unwrap()[0], edited);
    std::fs::write(source.path().join("SKILL.md"), "changed origin").unwrap();
    SkillCatalogue::refresh_origins(&catalogue.store).unwrap();
    assert_ne!(
        catalogue.store.skills().unwrap()[0].origin_fingerprint,
        package.origin_fingerprint
    );
    assert_eq!(
        std::fs::read_to_string(source.path().join("SKILL.md")).unwrap(),
        "changed origin"
    );
}
#[test]
fn definition_master_replacement_does_not_publish_offline_nonmaster_edits() {
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("SKILL.md"), "original").unwrap();
    let master = SkillCatalogue::new(database(), source.path().join("owned")).unwrap();
    let package = master
        .import(
            &source.path().join("SKILL.md"),
            "review",
            "Review",
            "native",
        )
        .unwrap();
    let canonical = master.store.capture_owned().unwrap().remove(0);
    let replica = DefinitionStore::new(database());
    replica.receive(&canonical).unwrap();
    let mut edited = package.clone();
    edited.name = "offline edit".into();
    replica.save_skill(&edited).unwrap();
    assert!(replica.capture_owned().unwrap().is_empty());
    assert_eq!(replica.skills().unwrap()[0].name, "offline edit");
    replica.receive(&canonical).unwrap();
    assert_eq!(replica.skills().unwrap()[0].name, "review");
    let mut proposal = canonical.clone();
    proposal.content["name"] = serde_json::json!("online edit");
    let accepted = master.store.edit_on_master(&proposal).unwrap();
    replica.receive(&accepted).unwrap();
    assert_eq!(master.store.skills().unwrap()[0].name, "online edit");
    assert_eq!(replica.skills().unwrap()[0].name, "online edit");
    assert!(master.store.edit_on_master(&proposal).is_err());
    let server = DefinitionStore::new(database());
    assert!(server
        .accept_master(&accepted.master_device_id, &accepted)
        .unwrap());
    assert!(!server
        .accept_master(&canonical.master_device_id, &canonical)
        .unwrap());
    assert!(server.accept_master("wrong-master", &accepted).is_err());
}
#[test]
fn package_paths_cannot_escape_materialization_or_alias_windows_files() {
    for path in [
        "../escape",
        "/absolute",
        "a\\..\\escape",
        "C:/escape",
        "a/CON.txt",
        "a/../b",
        "a/thing.",
    ] {
        assert!(files::validate_relative(path).is_err(), "{path}");
    }
}
