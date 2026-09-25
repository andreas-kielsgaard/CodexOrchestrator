//! Invocation-scoped grants to Orchid-owned MCP listeners.
//!
//! A grant states which product server one invocation may reach and with which credential. It is
//! provider-neutral launch intent: the selected provider decides how to configure the connection
//! and deliver the bearer. Listeners and tool authorization stay with their orchestration owners.
use crate::agent_sessions::ports::{RuntimeLaunchExtension, RuntimeManagedMcpServer};
use axum::http::{header, StatusCode};

const IMPLEMENTER_REPORTING_SCOPE: &str = "work_unit_implementer_reporting";
const IMPLEMENTER_REPORTING_TOOLS: [&str; 2] = [
    "submit_implementation_outcome",
    "complete_implementation_outcome",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManagedMcpGrant {
    server: RuntimeManagedMcpServer,
    sandbox_network_access: bool,
}

impl ManagedMcpGrant {
    pub(crate) fn plan_builder(
        server_url: &str,
        bearer: String,
        enabled_tools: &[String],
        required: bool,
    ) -> Self {
        Self::new("plan_builder", server_url, bearer, enabled_tools, required)
    }

    /// A fresh uniquely named connection, so it never collides with a provider's own servers.
    pub(crate) fn new(
        scope: &str,
        server_url: &str,
        bearer: String,
        enabled_tools: &[String],
        required: bool,
    ) -> Self {
        Self {
            server: RuntimeManagedMcpServer {
                name: format!("{scope}_{}", uuid::Uuid::new_v4().simple()),
                url: server_url.into(),
                bearer_token: Some(bearer),
                enabled_tools: Some(enabled_tools.to_vec()),
                required,
            },
            sandbox_network_access: false,
        }
    }

    /// The sole WorkspaceWrite exception: the exact same-Session Implementer reporting
    /// continuation must reach its token-protected loopback transport from inside the sandbox.
    pub(crate) fn work_unit_implementer_reporting(server_url: &str, bearer: String) -> Self {
        let tools = IMPLEMENTER_REPORTING_TOOLS.map(String::from);
        let mut grant = Self::new(IMPLEMENTER_REPORTING_SCOPE, server_url, bearer, &tools, true);
        grant.sandbox_network_access = true;
        grant
    }

    pub(crate) fn server(&self) -> &RuntimeManagedMcpServer {
        &self.server
    }

    pub(crate) fn apply(self, extension: &mut RuntimeLaunchExtension) {
        extension.managed_mcp_servers.push(self.server);
        extension.sandbox_network_access |= self.sandbox_network_access;
    }

    pub(crate) fn is_exact_work_unit_implementer_reporting_transport(&self) -> bool {
        let server = &self.server;
        self.sandbox_network_access
            && server.required
            && server
                .name
                .strip_prefix(IMPLEMENTER_REPORTING_SCOPE)
                .is_some_and(|suffix| suffix.len() > 1 && suffix.starts_with('_'))
            && !server.url.is_empty()
            && server.bearer_token.as_deref().is_some_and(|bearer| !bearer.is_empty())
            && server.enabled_tools.as_deref().is_some_and(|tools| {
                tools.iter().map(String::as_str).eq(IMPLEMENTER_REPORTING_TOOLS)
            })
    }
}

pub(crate) fn transport_denial<B>(
    expected: &str,
    allowed_host: &str,
    allowed_origins: &[String],
    request: &hyper::Request<B>,
) -> Option<StatusCode> {
    let valid = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.strip_prefix("Bearer ") == Some(expected));
    if !valid {
        return Some(StatusCode::UNAUTHORIZED);
    }
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(allowed_host)
    {
        return Some(StatusCode::FORBIDDEN);
    }
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        if !allowed_origins.iter().any(|allowed| allowed == origin) {
            return Some(StatusCode::FORBIDDEN);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_is_child_scoped_and_carries_only_accepted_intent() {
        let tools = vec!["submit_epic_plan_proposal".to_string()];
        let grant =
            ManagedMcpGrant::plan_builder("http://127.0.0.1:5555/mcp", "secret".into(), &tools, true);
        let mut extension = RuntimeLaunchExtension::default();
        grant.clone().apply(&mut extension);

        let server = &extension.managed_mcp_servers[0];
        assert!(server.name.starts_with("plan_builder_"));
        assert_eq!(server.bearer_token.as_deref(), Some("secret"));
        assert_eq!(server.enabled_tools.as_deref(), Some(tools.as_slice()));
        assert!(server.required);
        assert!(!extension.sandbox_network_access);
        assert!(extension.environment.is_empty());
        assert!(!format!("{grant:?}").contains("secret"));
        assert!(!grant.is_exact_work_unit_implementer_reporting_transport());
    }

    #[test]
    fn only_implementer_reporting_gets_the_workspace_write_network_exception() {
        let grant = ManagedMcpGrant::work_unit_implementer_reporting(
            "http://127.0.0.1:5555/mcp",
            "secret".into(),
        );
        assert!(grant.is_exact_work_unit_implementer_reporting_transport());
        let mut extension = RuntimeLaunchExtension::default();
        grant.clone().apply(&mut extension);
        assert!(extension.sandbox_network_access);
        assert_eq!(
            extension.managed_mcp_servers[0].enabled_tools.as_deref(),
            Some(IMPLEMENTER_REPORTING_TOOLS.map(String::from).as_slice())
        );

        let mut widened = grant.clone();
        widened.server.enabled_tools = Some(vec!["submit_implementation_outcome".into()]);
        assert!(!widened.is_exact_work_unit_implementer_reporting_transport());
        let mut unrestricted = grant;
        unrestricted.sandbox_network_access = false;
        assert!(!unrestricted.is_exact_work_unit_implementer_reporting_transport());
    }
}
