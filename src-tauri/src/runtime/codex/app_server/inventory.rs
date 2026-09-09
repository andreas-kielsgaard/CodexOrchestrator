//! Project native catalogue fields without exposing commands, URLs, headers or credentials.
use super::connection::Connection;
use crate::execution_configuration::{NativeCapabilityEntry, NativeCapabilityInventory};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn read(
    connection: &Connection,
    cwd: &Path,
    skill_roots: &[String],
) -> NativeCapabilityInventory {
    let mut inventory = NativeCapabilityInventory::default();
    let mut query = |method: &str, params: Value| match connection.call(method, params) {
        Ok(result) => Some(result),
        Err(_) => {
            inventory.limitations.push(format!(
                "The selected Codex runtime could not read {method}. Availability is unknown."
            ));
            None
        }
    };
    let skills = query("skills/list", json!({"cwds":[cwd],"forceReload":true}));
    let hooks = query("hooks/list", json!({"cwds":[cwd]}));
    let plugins = query(
        "plugin/list",
        json!({"cwds":[cwd],"marketplaceKinds":["local"]}),
    );
    let mut servers = Vec::new();
    let mut cursor = Value::Null;
    loop {
        let Some(page) = query(
            "mcpServerStatus/list",
            json!({"cursor":cursor,"detail":"toolsAndAuthOnly"}),
        ) else {
            break;
        };
        servers.extend(array(&page["data"]).cloned());
        cursor = page["nextCursor"].clone();
        if cursor.is_null() {
            break;
        }
    }
    for group in skills.iter().flat_map(|v| array(&v["data"])) {
        for skill in array(&group["skills"]) {
            let origin = if skill["path"].as_str().is_some_and(|p| {
                skill_roots
                    .iter()
                    .any(|root| Path::new(p).starts_with(root))
            }) {
                "orchestration"
            } else {
                skill["scope"].as_str().unwrap_or("native")
            };
            push(
                &mut inventory,
                text(&skill["name"]),
                "skill",
                origin,
                enabled(skill),
                "discovered by Codex",
            );
        }
        if array(&group["errors"]).next().is_some() {
            inventory.limitations.push(
                "Codex reported skill discovery errors; the catalogue may be incomplete.".into(),
            );
        }
    }
    for group in hooks.iter().flat_map(|v| array(&v["data"])) {
        for hook in array(&group["hooks"]) {
            push(
                &mut inventory,
                format!("{} ({})", text(&hook["eventName"]), text(&hook["key"])),
                "hook",
                &text(&hook["source"]),
                enabled(hook),
                &format!(
                    "{}; native trust: {}",
                    text(&hook["handlerType"]),
                    text(&hook["trustStatus"])
                ),
            );
        }
        if array(&group["errors"]).next().is_some() {
            inventory.limitations.push(
                "Codex reported hook discovery errors; the catalogue may be incomplete.".into(),
            );
        }
    }
    for marketplace in plugins.iter().flat_map(|v| array(&v["marketplaces"])) {
        for plugin in array(&marketplace["plugins"]).filter(|p| p["installed"] == true) {
            push(
                &mut inventory,
                text(&plugin["name"]),
                "plugin",
                &text(&marketplace["name"]),
                enabled(plugin),
                "installed; component execution support is not established by inventory",
            );
        }
    }
    for server in servers {
        let name = text(&server["name"]);
        let tools = server["tools"].as_object();
        push(
            &mut inventory,
            name.clone(),
            "mcp_server",
            "native",
            "configured",
            if tools.is_some_and(|v| !v.is_empty()) {
                "tool catalogue observed"
            } else {
                "no tools observed; execution support unknown"
            },
        );
        for tool in tools.into_iter().flat_map(|v| v.values()) {
            push(
                &mut inventory,
                format!("{name}/{}", text(&tool["name"])),
                "mcp_tool",
                "native",
                "exposed",
                "tool catalogue observed; execution is invocation-dependent",
            );
        }
    }
    inventory
        .entries
        .sort_by(|a, b| (&a.kind, &a.name).cmp(&(&b.kind, &b.name)));
    inventory
}

fn array(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
fn text(value: &Value) -> String {
    value.as_str().unwrap_or("unknown").into()
}
fn enabled(value: &Value) -> &'static str {
    if value["enabled"] == false {
        "disabled"
    } else {
        "enabled"
    }
}
fn push(
    inventory: &mut NativeCapabilityInventory,
    name: String,
    kind: &str,
    origin: &str,
    state: &str,
    support: &str,
) {
    inventory.entries.push(NativeCapabilityEntry {
        name,
        kind: kind.into(),
        origin: origin.into(),
        state: state.into(),
        support: support.into(),
    });
}
