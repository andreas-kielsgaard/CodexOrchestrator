use serde_json::Value;

#[test]
fn main_window_can_subscribe_to_agent_session_updates() {
    let capability: Value = serde_json::from_str(include_str!("../capabilities/default.json"))
        .expect("default capability is valid JSON");

    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    let permissions = capability["permissions"]
        .as_array()
        .expect("capability permissions");
    assert!(permissions.contains(&Value::String("core:event:allow-listen".into())));
    assert!(permissions.contains(&Value::String("core:event:allow-unlisten".into())));
}
