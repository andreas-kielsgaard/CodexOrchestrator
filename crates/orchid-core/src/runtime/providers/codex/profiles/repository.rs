//! Stored provider setup observations and invariant validation.
use super::*;
pub(super) fn load_pending_setup_attempts(
    connection: &Connection,
    profile_id: &str,
) -> Result<Vec<PendingSetupAttempt>, String> {
    let mut statement = connection
        .prepare(
            "SELECT attempt_id,profile_id,filesystem_identity,phase,deadline_at FROM native_codex_profile_setup_attempts WHERE profile_id=?1 AND state='pending' ORDER BY requested_at",
        )
        .map_err(|error| error.to_string())?;
    let attempts = statement
        .query_map(params![profile_id], |row| {
            let deadline: String = row.get(4)?;
            Ok(PendingSetupAttempt {
                attempt_id: row.get(0)?,
                profile_id: row.get(1)?,
                filesystem_identity: row.get(2)?,
                phase: SetupPhase::from_database(&row.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                deadline_at: DateTime::parse_from_rfc3339(&deadline)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(attempts)
}

pub(super) fn load_sandbox_adoption(
    connection: &Connection,
    profile_id: &str,
    identity: &str,
) -> Result<NativeProfileSandboxAdoption, String> {
    let adoption = connection
        .query_row(
            "SELECT filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed FROM native_codex_profile_sandbox_adoptions WHERE profile_id=?1",
            params![profile_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, i64>(8)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((
        stored_identity,
        executable,
        version,
        workspace,
        setup,
        correlation,
        observed_at,
        state,
        elevated,
    )) = adoption
    else {
        return Ok(NativeProfileSandboxAdoption {
            disposition: "not_verified".into(),
            executable: None,
            version: None,
            workspace_sandbox_supported: None,
            windows_sandbox_setup_supported: None,
            correlation_id: None,
            observed_at: None,
            elevated_mode_observed: None,
        });
    };
    if stored_identity != identity || state == "invalidated" {
        return Ok(NativeProfileSandboxAdoption {
            disposition: "invalidated".into(),
            executable: Some(executable),
            version: Some(version),
            workspace_sandbox_supported: Some(workspace != 0),
            windows_sandbox_setup_supported: Some(setup != 0),
            correlation_id: Some(correlation),
            observed_at: Some(observed_at),
            elevated_mode_observed: Some(elevated != 0),
        });
    }
    let adoption = NativeProfileSandboxAdoption {
        disposition: state,
        executable: Some(executable),
        version: Some(version),
        workspace_sandbox_supported: Some(workspace != 0),
        windows_sandbox_setup_supported: Some(setup != 0),
        correlation_id: Some(correlation),
        observed_at: Some(observed_at),
        elevated_mode_observed: Some(elevated != 0),
    };
    validate_sandbox_adoption(&adoption)?;
    Ok(adoption)
}

pub(super) fn validate_sandbox_adoption(
    adoption: &NativeProfileSandboxAdoption,
) -> Result<(), String> {
    let required = |value: &Option<String>| {
        value
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value)
    };
    if !matches!(
        adoption.disposition.as_str(),
        "verified" | "not_verified" | "invalidated"
    ) || !required(&adoption.executable)
        || !required(&adoption.version)
        || !required(&adoption.correlation_id)
        || adoption
            .observed_at
            .as_deref()
            .is_none_or(|value| DateTime::parse_from_rfc3339(value).is_err())
        || adoption.workspace_sandbox_supported.is_none()
        || adoption.windows_sandbox_setup_supported.is_none()
        || adoption.elevated_mode_observed.is_none()
        || (adoption.disposition == "verified"
            && (!adoption.workspace_sandbox_supported.unwrap()
                || !adoption.windows_sandbox_setup_supported.unwrap()
                || !adoption.elevated_mode_observed.unwrap()))
    {
        return Err("Native sandbox adoption evidence violates its durable invariant".into());
    }
    Ok(())
}

pub(super) fn load_sandbox_adoption_confirmation(
    connection: &Connection,
    profile_id: &str,
    identity: &str,
    adoption: &NativeProfileSandboxAdoption,
) -> Result<NativeProfileSandboxAdoptionConfirmation, String> {
    let confirmation = connection.query_row(
        "SELECT filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state FROM native_codex_profile_sandbox_adoption_confirmations WHERE profile_id=?1",
        params![profile_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
    ).optional().map_err(|error| error.to_string())?;
    let Some((stored_identity, adoption_correlation, correlation, confirmed_at, state)) =
        confirmation
    else {
        return Ok(NativeProfileSandboxAdoptionConfirmation {
            disposition: "not_confirmed".into(),
            correlation_id: None,
            confirmed_at: None,
        });
    };
    let matches_observation = stored_identity == identity
        && adoption.disposition == "verified"
        && adoption.correlation_id.as_deref() == Some(adoption_correlation.as_str());
    let confirmation = NativeProfileSandboxAdoptionConfirmation {
        disposition: if state == "confirmed" && matches_observation {
            "confirmed".into()
        } else {
            "invalidated".into()
        },
        correlation_id: Some(correlation),
        confirmed_at: Some(confirmed_at),
    };
    validate_sandbox_adoption_confirmation(&confirmation)?;
    Ok(confirmation)
}

pub(super) fn validate_sandbox_adoption_confirmation(
    confirmation: &NativeProfileSandboxAdoptionConfirmation,
) -> Result<(), String> {
    if confirmation.disposition == "not_confirmed"
        && confirmation.correlation_id.is_none()
        && confirmation.confirmed_at.is_none()
    {
        return Ok(());
    }
    if !matches!(
        confirmation.disposition.as_str(),
        "confirmed" | "invalidated"
    ) || confirmation
        .correlation_id
        .as_deref()
        .is_none_or(|value| value.is_empty() || value.trim() != value)
        || confirmation
            .confirmed_at
            .as_deref()
            .is_none_or(|value| DateTime::parse_from_rfc3339(value).is_err())
    {
        return Err("Native sandbox adoption confirmation violates its durable invariant".into());
    }
    Ok(())
}

pub(super) fn load_pending_full_access_canary(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<PendingFullAccessCanary>, String> {
    connection
        .query_row(
            "SELECT attempt_id,profile_id,filesystem_identity,executable,version,sentinel_path,authorization_correlation_id,authorization_version,correlation_id,deadline_at FROM native_codex_profile_full_access_canaries WHERE profile_id=?1 AND state='pending'",
            params![profile_id],
            |row| Ok(PendingFullAccessCanary {
                attempt_id: row.get(0)?,
                profile_id: row.get(1)?,
                filesystem_identity: row.get(2)?,
                executable: row.get(3)?,
                version: row.get(4)?,
                sentinel_path: PathBuf::from(row.get::<_, String>(5)?),
                authorization_correlation_id: row.get(6)?,
                authorization_version: row.get(7)?,
                correlation_id: row.get(8)?,
                deadline_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc),
            }),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub(super) fn valid_durable_timestamp(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && DateTime::parse_from_rfc3339(value).is_ok()
}

pub(super) fn load_danger_authorization(
    connection: &Connection,
    profile_id: &str,
    filesystem_identity: &str,
) -> Result<NativeProfileDangerAuthorization, String> {
    let row = connection
        .query_row(
            "SELECT filesystem_identity,authority_scope,authority_version,authorization_correlation_id,authorized_at,revoked_at FROM native_codex_profile_mode_authorizations WHERE profile_id=?1 AND mode='danger_full_access'",
            params![profile_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((recorded_identity, scope, version, correlation_id, authorized_at, revoked_at)) = row
    else {
        return Ok(NativeProfileDangerAuthorization {
            disposition: "not_authorized".into(),
            authority_scope: None,
            authority_version: None,
            correlation_id: None,
            authorized_at: None,
            revoked_at: None,
        });
    };
    let legacy = scope == "filesystem_only"
        && version == "danger-full-access/filesystem-only/v1"
        && correlation_id.is_none();
    let current = scope == DANGER_AUTHORITY_SCOPE
        && version == DANGER_AUTHORITY_VERSION
        && correlation_id
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value);
    if (!legacy && !current)
        || !valid_durable_timestamp(&authorized_at)
        || revoked_at
            .as_deref()
            .is_some_and(|value| !valid_durable_timestamp(value))
    {
        return Err("Native danger authorization violates its durable invariant".into());
    }
    let disposition = if recorded_identity != filesystem_identity {
        "foreign"
    } else if revoked_at.is_some() {
        "revoked"
    } else if current {
        "authorized"
    } else {
        "legacy_insufficient"
    };
    Ok(NativeProfileDangerAuthorization {
        disposition: disposition.into(),
        authority_scope: Some(scope),
        authority_version: Some(version),
        correlation_id,
        authorized_at: Some(authorized_at),
        revoked_at,
    })
}

pub(super) fn load_full_access_canary_attempt(
    connection: &Connection,
    profile_id: &str,
    filesystem_identity: &str,
) -> Result<NativeProfileFullAccessCanaryAttempt, String> {
    let row = connection.query_row(
        "SELECT filesystem_identity,state,authorization_version,authorization_correlation_id,correlation_id,requested_at,launch_accepted_at,deadline_at,settled_at,process_activity,provider_activity,terminal_classification,terminal_exit_code,receipt_observed,cleanup_disposition FROM native_codex_profile_full_access_canaries WHERE profile_id=?1 ORDER BY requested_at DESC,attempt_id DESC LIMIT 1",
        params![profile_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, Option<String>>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, String>(5)?, row.get::<_, Option<String>>(6)?, row.get::<_, Option<String>>(7)?, row.get::<_, Option<String>>(8)?, row.get::<_, String>(9)?, row.get::<_, String>(10)?, row.get::<_, String>(11)?, row.get::<_, Option<i32>>(12)?, row.get::<_, i64>(13)?, row.get::<_, String>(14)?)),
    ).optional().map_err(|error| error.to_string())?;
    let Some((
        recorded_identity,
        disposition,
        authorization_version,
        authorization_correlation_id,
        correlation_id,
        requested_at,
        launch_accepted_at,
        deadline_at,
        settled_at,
        process_activity,
        provider_activity,
        terminal_classification,
        terminal_exit_code,
        receipt_observed,
        cleanup_disposition,
    )) = row
    else {
        return Ok(NativeProfileFullAccessCanaryAttempt {
            disposition: "not_requested".into(),
            authorization_version: None,
            authorization_correlation_id: None,
            correlation_id: None,
            requested_at: None,
            launch_accepted_at: None,
            deadline_at: None,
            settled_at: None,
            process_activity: "unobserved".into(),
            provider_activity: "unobserved".into(),
            terminal_classification: "not_observed".into(),
            terminal_exit_code: None,
            receipt_observed: false,
            cleanup_disposition: "not_observed".into(),
        });
    };
    let valid_state = [
        "pending",
        "passed",
        "launch_failed",
        "terminal_failed",
        "timed_out",
        "cancelled",
        "recovered_unobserved",
        "cleanup_failed",
        "legacy_unverified",
    ]
    .contains(&disposition.as_str());
    let valid_process =
        ["unobserved", "launch_accepted", "terminal_observed"].contains(&process_activity.as_str());
    let valid_terminal = [
        "not_observed",
        "exit_code",
        "receipt_missing",
        "launch_failed",
        "timed_out",
        "cancelled",
        "recovered_unobserved",
        "cleanup_failed",
        "legacy_unverified",
    ]
    .contains(&terminal_classification.as_str());
    let valid_cleanup =
        ["pending", "removed", "failed", "not_observed"].contains(&cleanup_disposition.as_str());
    let valid_optional_timestamp =
        |value: &Option<String>| value.as_deref().is_none_or(valid_durable_timestamp);
    let current_authority = authorization_version.as_deref() == Some(DANGER_AUTHORITY_VERSION)
        && authorization_correlation_id
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value)
        && correlation_id
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value);
    let requested = DateTime::parse_from_rfc3339(&requested_at)
        .map_err(|_| "Native full-access canary violates its durable invariant")?;
    let launch = launch_accepted_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| "Native full-access canary violates its durable invariant")?;
    let deadline = deadline_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| "Native full-access canary violates its durable invariant")?;
    let settled = settled_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| "Native full-access canary violates its durable invariant")?;
    let timestamps_ordered = launch.as_ref().is_none_or(|launch| launch >= &requested)
        && deadline
            .as_ref()
            .is_none_or(|deadline| deadline >= &requested)
        && settled.as_ref().is_none_or(|settled| {
            settled >= &requested && launch.as_ref().is_none_or(|launch| settled >= launch)
        });
    let accepted_or_unobserved = match launch_accepted_at.is_some() {
        true => process_activity == "launch_accepted" || process_activity == "terminal_observed",
        false => process_activity == "unobserved",
    };
    let interrupted_process = match launch_accepted_at.is_some() {
        true => process_activity == "launch_accepted",
        false => process_activity == "unobserved",
    };
    let cleanup_settled = cleanup_disposition == "removed" || cleanup_disposition == "failed";
    let pending_shape = settled_at.is_none()
        && cleanup_disposition == "pending"
        && terminal_classification == "not_observed"
        && terminal_exit_code.is_none()
        && receipt_observed == 0
        && accepted_or_unobserved;
    let terminal_failed_shape = launch_accepted_at.is_some()
        && process_activity == "terminal_observed"
        && settled_at.is_some()
        && cleanup_settled
        && ((terminal_classification == "receipt_missing" && receipt_observed == 0)
            || (terminal_classification == "exit_code" && terminal_exit_code.is_some())
            || (terminal_classification == "not_observed" && terminal_exit_code.is_none()));
    let cancelled_shape = settled_at.is_some()
        && cleanup_settled
        && terminal_classification == disposition
        && terminal_exit_code.is_none()
        && receipt_observed == 0
        && interrupted_process;
    let cleanup_failed_shape = launch_accepted_at.is_some()
        && process_activity == "terminal_observed"
        && settled_at.is_some()
        && terminal_classification == "cleanup_failed"
        && receipt_observed == 1
        && cleanup_disposition == "failed";
    let legacy_shape = disposition == "legacy_unverified"
        && terminal_classification == "legacy_unverified"
        && process_activity == "unobserved"
        && authorization_version.is_none()
        && authorization_correlation_id.is_none()
        && correlation_id.is_none()
        && launch_accepted_at.is_none()
        && deadline_at.is_none()
        && settled_at.is_some()
        && terminal_exit_code.is_none()
        && receipt_observed == 0
        && cleanup_disposition == "not_observed";
    let legacy_recovered_shape = disposition == "recovered_unobserved"
        && authorization_version.is_none()
        && authorization_correlation_id.is_none()
        && correlation_id.is_none()
        && launch_accepted_at.is_none()
        && deadline_at.is_none()
        && process_activity == "unobserved"
        && terminal_classification == "recovered_unobserved"
        && terminal_exit_code.is_none()
        && receipt_observed == 0
        && cleanup_disposition == "not_observed";
    if !valid_state
        || !valid_process
        || provider_activity != "unobserved"
        || !valid_terminal
        || !valid_cleanup
        || !valid_durable_timestamp(&requested_at)
        || !valid_optional_timestamp(&launch_accepted_at)
        || !valid_optional_timestamp(&deadline_at)
        || !valid_optional_timestamp(&settled_at)
        || receipt_observed != 0 && receipt_observed != 1
        || !timestamps_ordered
        || (!matches!(
            disposition.as_str(),
            "legacy_unverified" | "recovered_unobserved"
        ) && deadline_at.is_none())
        || (!matches!(
            disposition.as_str(),
            "legacy_unverified" | "recovered_unobserved"
        ) && !current_authority)
        || (disposition == "pending" && !pending_shape)
        || (disposition == "passed"
            && (launch_accepted_at.is_none()
                || process_activity != "terminal_observed"
                || settled_at.is_none()
                || receipt_observed != 1
                || cleanup_disposition != "removed"
                || !((terminal_classification == "exit_code" && terminal_exit_code.is_some())
                    || (terminal_classification == "not_observed"
                        && terminal_exit_code.is_none()))))
        || (disposition == "launch_failed"
            && (launch_accepted_at.is_some()
                || settled_at.is_none()
                || process_activity != "unobserved"
                || terminal_classification != "launch_failed"
                || terminal_exit_code.is_some()
                || receipt_observed != 0
                || !cleanup_settled))
        || (disposition == "terminal_failed" && !terminal_failed_shape)
        || (matches!(disposition.as_str(), "timed_out" | "cancelled") && !cancelled_shape)
        || (disposition == "recovered_unobserved" && !(cancelled_shape || legacy_recovered_shape))
        || (disposition == "cleanup_failed" && !cleanup_failed_shape)
        || (disposition == "legacy_unverified" && !legacy_shape)
    {
        return Err("Native full-access canary violates its durable invariant".into());
    }
    if recorded_identity != filesystem_identity {
        return Ok(NativeProfileFullAccessCanaryAttempt {
            disposition: "recovered_unobserved".into(),
            authorization_version: None,
            authorization_correlation_id: None,
            correlation_id: None,
            requested_at: None,
            launch_accepted_at: None,
            deadline_at: None,
            settled_at: None,
            process_activity: "unobserved".into(),
            provider_activity: "unobserved".into(),
            terminal_classification: "recovered_unobserved".into(),
            terminal_exit_code: None,
            receipt_observed: false,
            cleanup_disposition: "not_observed".into(),
        });
    }
    Ok(NativeProfileFullAccessCanaryAttempt {
        disposition,
        authorization_version,
        authorization_correlation_id,
        correlation_id,
        requested_at: Some(requested_at),
        launch_accepted_at,
        deadline_at,
        settled_at,
        process_activity,
        provider_activity,
        terminal_classification,
        terminal_exit_code,
        receipt_observed: receipt_observed != 0,
        cleanup_disposition,
    })
}

pub(super) fn load_pending_login_attempt(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<PendingLoginAttempt>, String> {
    connection.query_row(
        "SELECT attempt_id,profile_id,filesystem_identity FROM native_codex_profile_login_attempts WHERE profile_id=?1 AND state='pending'",
        params![profile_id],
        |row| Ok(PendingLoginAttempt {
            attempt_id: row.get(0)?,
            profile_id: row.get(1)?,
            filesystem_identity: row.get(2)?,
        }),
    ).optional().map_err(|error| error.to_string())
}

pub(super) fn load_pending_mcp_probe(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<NativeMcpReportingProbeAuthority>, String> {
    connection
        .query_row(
            "SELECT correlation_id,expected_capability,expected_server,expected_tool,expected_probe_root FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND state='pending'",
            params![profile_id],
            |row| Ok(NativeMcpReportingProbeAuthority {
                profile_id: profile_id.into(),
                correlation_id: row.get(0)?,
                capability: row.get(1)?,
                server: row.get(2)?,
                tool: row.get(3)?,
                probe_root: PathBuf::from(row.get::<_, String>(4)?),
            }),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub(super) fn load_profiles(connection: &Connection) -> Result<Vec<StoredProfile>, String> {
    let mut statement = connection.prepare("SELECT p.id,p.canonical_home_path,p.filesystem_identity,p.ownership,p.lifecycle,p.selected_at,r.authentication,r.sandbox_initialization,r.workspace_write_canary,r.danger_full_access_canary,r.mcp_reporting,e.selected_mode,a.filesystem_identity,a.revoked_at,(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='authentication'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='sandbox'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='canary'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='mcp_reporting'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='continuity'),(SELECT detail FROM native_codex_profile_attentions x WHERE x.profile_id=p.id AND x.concern='cli'),(SELECT state FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT browser_handoff FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT requested_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT launch_accepted_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT settled_at FROM native_codex_profile_login_attempts l WHERE l.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT phase FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT state FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT executable FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT version FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT workspace_sandbox_supported FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT correlation_id FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT requested_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT launch_accepted_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT deadline_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT settled_at FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT terminal_classification FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1),(SELECT terminal_exit_code FROM native_codex_profile_setup_attempts s WHERE s.profile_id=p.id ORDER BY requested_at DESC,attempt_id DESC LIMIT 1) FROM native_codex_profiles p JOIN native_codex_profile_readiness r ON r.profile_id=p.id JOIN native_codex_profile_execution_modes e ON e.profile_id=p.id LEFT JOIN native_codex_profile_mode_authorizations a ON a.profile_id=p.id AND a.mode='danger_full_access' ORDER BY p.created_at").map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let identity: String = row.get(2)?;
            let authorization_identity: Option<String> = row.get(12)?;
            let authorization_revoked: Option<String> = row.get(13)?;
            Ok(StoredProfile {
                id: row.get(0)?,
                home: PathBuf::from(row.get::<_, String>(1)?),
                identity: identity.clone(),
                ownership: Ownership::parse(&row.get::<_, String>(3)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                lifecycle: Lifecycle::parse(&row.get::<_, String>(4)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                selected: row.get::<_, Option<String>>(5)?.is_some(),
                personality: None,
                execution: NativeProfileExecution {
                    selected_mode: ExecutionMode::parse(&row.get::<_, String>(11)?)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    danger_full_access_authorized: false,
                    danger_authorization: NativeProfileDangerAuthorization {
                        disposition: if authorization_identity == Some(identity)
                            && authorization_revoked.is_none()
                        {
                            "not_authorized".into()
                        } else {
                            "not_authorized".into()
                        },
                        authority_scope: None,
                        authority_version: None,
                        correlation_id: None,
                        authorized_at: None,
                        revoked_at: None,
                    },
                },
                login_attempt: NativeProfileLoginAttempt {
                    disposition: row
                        .get::<_, Option<String>>(20)?
                        .unwrap_or_else(|| "not_requested".into()),
                    browser_handoff: row
                        .get::<_, Option<String>>(21)?
                        .unwrap_or_else(|| "unobserved".into()),
                    requested_at: row.get(22)?,
                    launch_accepted_at: row.get(23)?,
                    settled_at: row.get(24)?,
                },
                setup_attempt: NativeProfileSetupAttempt {
                    phase: row
                        .get::<_, Option<String>>(25)?
                        .unwrap_or_else(|| "not_requested".into()),
                    disposition: row
                        .get::<_, Option<String>>(26)?
                        .unwrap_or_else(|| "not_requested".into()),
                    executable: row.get(27)?,
                    version: row.get(28)?,
                    workspace_sandbox_supported: row
                        .get::<_, Option<i64>>(29)?
                        .map(|value| value != 0),
                    correlation_id: row.get(30)?,
                    requested_at: row.get(31)?,
                    launch_accepted_at: row.get(32)?,
                    deadline_at: row.get(33)?,
                    settled_at: row.get(34)?,
                    terminal_classification: row
                        .get::<_, Option<String>>(35)?
                        .unwrap_or_else(|| "not_observed".into()),
                    terminal_exit_code: row.get(36)?,
                },
                sandbox_adoption: NativeProfileSandboxAdoption {
                    disposition: "not_verified".into(),
                    executable: None,
                    version: None,
                    workspace_sandbox_supported: None,
                    windows_sandbox_setup_supported: None,
                    correlation_id: None,
                    observed_at: None,
                    elevated_mode_observed: None,
                },
                sandbox_adoption_confirmation: NativeProfileSandboxAdoptionConfirmation {
                    disposition: "not_confirmed".into(),
                    correlation_id: None,
                    confirmed_at: None,
                },
                full_access_canary_attempt: NativeProfileFullAccessCanaryAttempt {
                    disposition: "not_requested".into(),
                    authorization_version: None,
                    authorization_correlation_id: None,
                    correlation_id: None,
                    requested_at: None,
                    launch_accepted_at: None,
                    deadline_at: None,
                    settled_at: None,
                    process_activity: "unobserved".into(),
                    provider_activity: "unobserved".into(),
                    terminal_classification: "not_observed".into(),
                    terminal_exit_code: None,
                    receipt_observed: false,
                    cleanup_disposition: "not_observed".into(),
                },
                readiness: NativeProfileReadiness {
                    authentication: row.get(6)?,
                    sandbox_initialization: row.get(7)?,
                    workspace_write_canary: row.get(8)?,
                    danger_full_access_canary: row.get(9)?,
                    mcp_reporting: row.get(10)?,
                    attentions: NativeProfileAttentions {
                        authentication: row.get(14)?,
                        sandbox: row.get(15)?,
                        canary: row.get(16)?,
                        mcp_reporting: row.get(17)?,
                        continuity: row.get(18)?,
                        cli: row.get(19)?,
                    },
                },
            })
        })
        .map_err(|error| error.to_string())?;
    let mut profiles = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for profile in &mut profiles {
        profile.personality = load_profile_personality(connection, &profile.id)?;
        validate_setup_attempt(&profile.setup_attempt)?;
        profile.execution.danger_authorization =
            load_danger_authorization(connection, &profile.id, &profile.identity)?;
        profile.execution.danger_full_access_authorized =
            profile.execution.danger_authorization.disposition == "authorized";
        profile.sandbox_adoption =
            load_sandbox_adoption(connection, &profile.id, &profile.identity)?;
        profile.full_access_canary_attempt =
            load_full_access_canary_attempt(connection, &profile.id, &profile.identity)?;
        profile.sandbox_adoption_confirmation = load_sandbox_adoption_confirmation(
            connection,
            &profile.id,
            &profile.identity,
            &profile.sandbox_adoption,
        )?;
    }
    Ok(profiles)
}

pub(super) fn personality_database(personality: Option<CodexPersonality>) -> &'static str {
    match personality {
        None => "inherit",
        Some(CodexPersonality::None) => "none",
        Some(CodexPersonality::Friendly) => "friendly",
        Some(CodexPersonality::Pragmatic) => "pragmatic",
    }
}

pub(super) fn load_profile_personality(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<CodexPersonality>, String> {
    let stored = connection
        .query_row(
            "SELECT personality FROM native_codex_profile_preferences WHERE profile_id=?1",
            params![profile_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match stored.as_deref().unwrap_or("inherit") {
        "inherit" => Ok(None),
        "none" => Ok(Some(CodexPersonality::None)),
        "friendly" => Ok(Some(CodexPersonality::Friendly)),
        "pragmatic" => Ok(Some(CodexPersonality::Pragmatic)),
        _ => Err("Stored native Codex personality is invalid".into()),
    }
}

pub(super) fn validate_setup_attempt(attempt: &NativeProfileSetupAttempt) -> Result<(), String> {
    let required = |value: &Option<String>| {
        value
            .as_deref()
            .is_some_and(|value| !value.is_empty() && value.trim() == value)
    };
    match attempt.terminal_classification.as_str() {
        "policy_unsupported" => {
            if attempt.disposition != "policy_unsupported"
                || !matches!(
                    attempt.phase.as_str(),
                    "sandbox_initialization" | "workspace_write_canary"
                )
                || attempt.workspace_sandbox_supported != Some(false)
                || !required(&attempt.executable)
                || !required(&attempt.version)
                || !required(&attempt.correlation_id)
                || !required(&attempt.requested_at)
                || !required(&attempt.deadline_at)
                || !required(&attempt.settled_at)
                || attempt.launch_accepted_at.is_some()
                || attempt.terminal_exit_code.is_some()
            {
                return Err(
                    "Native policy-unsupported setup attempt violates its durable invariant".into(),
                );
            }
        }
        "receipt_missing" => {
            if attempt.disposition != "terminal_failed"
                || attempt.phase != "workspace_write_canary"
                || attempt.workspace_sandbox_supported != Some(true)
                || !required(&attempt.executable)
                || !required(&attempt.version)
                || !required(&attempt.correlation_id)
                || !required(&attempt.requested_at)
                || !required(&attempt.launch_accepted_at)
                || !required(&attempt.deadline_at)
                || !required(&attempt.settled_at)
            {
                return Err(
                    "Native receipt-missing setup attempt violates its durable invariant".into(),
                );
            }
        }
        _ => return Ok(()),
    }
    let requested = DateTime::parse_from_rfc3339(attempt.requested_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid request timestamp")?;
    let deadline = DateTime::parse_from_rfc3339(attempt.deadline_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid deadline timestamp")?;
    let settled = DateTime::parse_from_rfc3339(attempt.settled_at.as_deref().unwrap())
        .map_err(|_| "Native setup attempt has an invalid settlement timestamp")?;
    if deadline < requested || settled < requested {
        return Err("Native setup attempt has contradictory timestamps".into());
    }
    if let Some(accepted_at) = attempt.launch_accepted_at.as_deref() {
        let accepted = DateTime::parse_from_rfc3339(accepted_at)
            .map_err(|_| "Native setup attempt has an invalid launch timestamp")?;
        if accepted < requested || settled < accepted {
            return Err("Native setup attempt has contradictory launch timestamps".into());
        }
    }
    Ok(())
}
