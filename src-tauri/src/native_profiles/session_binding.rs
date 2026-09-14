//! Native home continuity and launch-only environment binding for Sessions.
use super::*;

impl crate::agent_sessions::ports::ImportHomeSource for NativeProfileService {
    fn selected_import_home(&self) -> Result<crate::agent_sessions::imports::ImportHome, String> {
        let home = self.resolve_session_home()?;
        Ok(crate::agent_sessions::imports::ImportHome {
            profile_id: home.profile_id,
            filesystem_identity: home.filesystem_identity,
            path: home.home.to_string_lossy().into_owned(),
        })
    }
}

pub(crate) fn insert_import_binding(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    home: &crate::agent_sessions::imports::ImportHome,
    at: &str,
) -> Result<(), String> {
    // Check selection and continuity inside the materialization transaction.
    let valid: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM native_codex_profiles WHERE id=?1 AND filesystem_identity=?2 AND selected_at IS NOT NULL AND lifecycle='active')",
        params![home.profile_id, home.filesystem_identity], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !valid {
        return Err("Selected native profile continuity changed during import".into());
    }
    tx.execute("INSERT INTO agent_session_native_profile_bindings(session_id,profile_id,filesystem_identity,bound_at) VALUES (?1,?2,?3,?4)",
        params![session_id, home.profile_id, home.filesystem_identity, at]).map_err(|e| e.to_string())?;
    Ok(())
}
impl NativeProfileService {
    pub(crate) fn resolve_session_home(&self) -> Result<ResolvedNativeCodexHome, String> {
        let profile = self.read("resolve Session native home", |connection| {
            load_profiles(connection)?
                .into_iter()
                .find(|profile| profile.selected)
                .ok_or_else(|| "No native Codex home is selected".to_string())
        })?;
        let profile = self.require_selected_active(&profile.id)?;
        if profile.lifecycle != Lifecycle::Active {
            return Err(
                "The selected native Codex home lost continuity and must be registered again"
                    .into(),
            );
        }
        let lifecycle = validate_profile(&profile);
        if lifecycle != Lifecycle::Active {
            self.record_lifecycle(&profile.id, lifecycle)?;
            return Err("The selected native Codex home no longer has validated continuity".into());
        }
        let readiness = &profile.readiness;
        Ok(ResolvedNativeCodexHome {
            profile_id: profile.id,
            filesystem_identity: profile.identity,
            home: profile.home,
            execution_mode: profile.execution.selected_mode,
            readiness: readiness.clone(),
        })
    }

    pub(super) fn prepare_managed_agent_session_launch(
        &self,
        session_id: &str,
        invocation_id: &str,
        resuming: bool,
        extension: Option<crate::agent_sessions::ports::RuntimeLaunchExtension>,
    ) -> Result<crate::agent_sessions::ports::RuntimeLaunchExtension, String> {
        let mut extension = extension.unwrap_or_default();
        if extension
            .environment
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("CODEX_HOME"))
        {
            return Err(
                "Only the application-selected native profile may supply CODEX_HOME".into(),
            );
        }
        let resolved = self.resolve_session_home()?;
        self.write("bind managed Session native profile", |transaction| {
        let binding = transaction
            .query_row(
                "SELECT profile_id,filesystem_identity FROM agent_session_native_profile_bindings WHERE session_id=?1",
                params![session_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| format!("Unable to load managed Agent Session native profile binding: {error}"))?;
        match binding {
            Some((profile_id, identity)) => {
                if profile_id != resolved.profile_id || identity != resolved.filesystem_identity {
                    return Err("Managed Agent Session native profile continuity no longer matches the selected ready profile".into());
                }
            }
            None => {
                if resuming {
                    return Err(
                        "Managed Agent Session resume requires a durable native profile binding"
                            .into(),
                    );
                }
                transaction.execute(
                    "INSERT INTO agent_session_native_profile_bindings (session_id,profile_id,filesystem_identity,bound_at) VALUES (?1,?2,?3,?4)",
                    params![session_id, resolved.profile_id, resolved.filesystem_identity, Utc::now().to_rfc3339()],
                ).map_err(|error| format!("Unable to persist managed Agent Session native profile binding: {error}"))?;
            }
        }
        let mode = if resuming { "resume" } else { "start" };
        let provenance = transaction
            .query_row(
                "SELECT session_id,profile_id,filesystem_identity,environment_key,invocation_mode FROM agent_session_native_profile_launch_provenance WHERE invocation_id=?1",
                params![invocation_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
            )
            .optional()
            .map_err(|error| format!("Unable to load managed Agent Session native launch provenance: {error}"))?;
        match provenance {
            Some((bound_session, profile_id, identity, key, bound_mode)) => {
                if bound_session != session_id
                    || profile_id != resolved.profile_id
                    || identity != resolved.filesystem_identity
                    || key != "CODEX_HOME"
                    || bound_mode != mode
                {
                    return Err("Managed Agent Session native launch provenance conflicts with its durable profile binding".into());
                }
            }
            None => {
                transaction.execute(
                    "INSERT INTO agent_session_native_profile_launch_provenance (invocation_id,session_id,profile_id,filesystem_identity,environment_key,invocation_mode,prepared_at) VALUES (?1,?2,?3,?4,'CODEX_HOME',?5,?6)",
                    params![invocation_id, session_id, resolved.profile_id, resolved.filesystem_identity, mode, Utc::now().to_rfc3339()],
                ).map_err(|error| format!("Unable to persist managed Agent Session native launch provenance: {error}"))?;
            }
        }
        Ok(())
        })?;
        extension.environment.push((
            "CODEX_HOME".into(),
            resolved.home.to_string_lossy().into_owned(),
        ));
        Ok(extension)
    }
}
