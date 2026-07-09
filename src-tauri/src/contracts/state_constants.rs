pub(crate) const DASHBOARD_GROUPS: [(&str, &str); 5] = [
    ("needs_action_now", "Needs action now"),
    ("review_decide", "Review / decide"),
    ("working", "Working"),
    ("waiting", "Waiting"),
    ("later", "Later"),
];

pub(crate) const EXECUTION_STATES: [&str; 8] = [
    "draft",
    "queued",
    "running",
    "blocked",
    "completed",
    "failed",
    "abandoned",
    "archived",
];

pub(crate) const ATTENTION_STATES: [&str; 7] = [
    "needs_action_now",
    "needs_review",
    "waiting_on_agent",
    "waiting_on_external",
    "consider_later",
    "snoozed",
    "reference_only",
];

pub(crate) const PRIORITIES: [&str; 3] = ["low", "normal", "high"];
