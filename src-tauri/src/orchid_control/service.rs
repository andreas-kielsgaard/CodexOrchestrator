pub(crate) use orchid_core::orchid_control::{required, OrchidControl};
use std::collections::{BTreeMap,BTreeSet};
pub(crate) fn tool_map()->BTreeMap<String,BTreeSet<String>> {
 let mut tools=orchid_core::orchid_control::tool_map();
 tools.extend([
        (
            "voice_session",
            [
                "delegate_work",
                "report_outcome",
                "publish",
                "observe_sessions",
                "read_context",
                "report_progress",
                "request_output",
                "route_input",
            ]
            .as_slice(),
        ),
].into_iter().map(|(server,tools):(&str,&[&str])|(server.into(),tools.iter().map(|s|s.to_string()).collect())));
 tools
}
