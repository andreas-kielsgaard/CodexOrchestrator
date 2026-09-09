//! Human-readable scope for native approval choices; response payloads stay unchanged.
use serde_json::{json, Value};

pub(super) fn project(decision: Value) -> Value {
    let (label, description, scope) = match decision.as_str() {
        Some("accept") => ("Allow once", "Run this command once.", None),
        Some("acceptForSession") => (
            "Allow for session",
            "Allow matching commands for this session.",
            None,
        ),
        Some("decline") => (
            "Decline",
            "Do not run this command; the agent can continue.",
            None,
        ),
        Some("cancel") => (
            "Cancel turn",
            "Do not run this command and stop the turn.",
            None,
        ),
        _ if decision.get("acceptWithExecpolicyAmendment").is_some() => (
            "Allow and save rule",
            "Allow this command and future commands matching the saved rule.",
            Some(
                decision["acceptWithExecpolicyAmendment"]["execpolicy_amendment"]
                    .as_array()
                    .map(|args| {
                        args.iter()
                            .map(Value::to_string)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_else(|| decision.to_string()),
            ),
        ),
        _ if decision.get("applyNetworkPolicyAmendment").is_some() => {
            let rule = &decision["applyNetworkPolicyAmendment"]["network_policy_amendment"];
            let (label, description) = match rule["action"].as_str() {
                Some("allow") => (
                    "Save allow rule",
                    "Allow future network access to this host.",
                ),
                Some("deny") => ("Save deny rule", "Deny future network access to this host."),
                _ => (
                    "Save network rule",
                    "Apply the offered network rule for this host.",
                ),
            };
            (
                label,
                description,
                Some(
                    rule["host"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| rule.to_string()),
                ),
            )
        }
        _ => (
            "Apply offered decision",
            "Review the decision details before applying.",
            Some(decision.to_string()),
        ),
    };
    json!({"label":label,"description":description,"scope":scope,"response":{"decision":decision}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_rules_explain_scope_without_changing_the_offered_decision() {
        let command =
            json!({"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["git","status"]}});
        let choice = project(command.clone());
        assert_eq!(choice["label"], "Allow and save rule");
        assert_eq!(choice["scope"], "\"git\" \"status\"");
        assert_eq!(choice["response"]["decision"], command);
        for (action, label) in [("allow", "Save allow rule"), ("deny", "Save deny rule")] {
            let decision = json!({"applyNetworkPolicyAmendment":{"network_policy_amendment":{"action":action,"host":"example.test"}}});
            let choice = project(decision.clone());
            assert_eq!(choice["label"], label);
            assert_eq!(choice["scope"], "example.test");
            assert_eq!(choice["response"]["decision"], decision);
        }
    }
}
