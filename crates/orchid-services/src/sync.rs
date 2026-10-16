use orchid_core::{
    agent_sessions::{domain::AgentSessionId, repository::SqliteAgentSessionRepository},
    synchronization::{repository::ReplicaStore, ServerConnection, SessionChange},
};
pub async fn synchronize_once(
    client: &reqwest::Client,
    connection: &ServerConnection,
    store: &ReplicaStore,
    repository: &SqliteAgentSessionRepository,
) -> Result<Vec<String>, String> {
    let base = connection.url.trim_end_matches('/');
    let device = store.device_id()?;
    let (mut sent, mut received) = store.cursor(base)?;
    // The source has already durably stopped its queue. Retrying activation cannot restart it.
    for (id, destination) in repository.pending_handoffs()? {
        let change = repository.export_current_session(
            &AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?,
            device.clone(),
        )?;
        let command = orchid_core::agent_sessions::client::HostCommand {
            method: "acceptHandoff".into(),
            input: serde_json::to_value(change).map_err(|e| e.to_string())?,
        };
        let outcome = client
            .post(format!("{base}/v1/devices/{destination}/commands"))
            .bearer_auth(&connection.token)
            .json(&command)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status);
        match outcome {
            Ok(_) => repository.acknowledge_handoff(&id, &destination)?,
            Err(error) => eprintln!("Ownership handoff {id} remains pending: {error}"),
        }
    }
    for (id, sequence) in store.pending_sessions(sent)? {
        if repository
            .execution_owner(&AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?)?
            .is_some_and(|owner| owner != device)
        {
            sent = sequence;
            store.acknowledge(base, sent, received)?;
            continue;
        }
        let change = repository.export_session(
            &AgentSessionId::new(id).map_err(|e| e.to_string())?,
            device.clone(),
            sequence,
        )?;
        client
            .post(format!("{base}/v1/sync/{device}"))
            .bearer_auth(&connection.token)
            .json(&change)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        sent = sequence;
        store.acknowledge(base, sent, received)?;
    }
    let changes: Vec<(u64, SessionChange)> = client
        .get(format!("{base}/v1/changes?after={received}"))
        .bearer_auth(&connection.token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let mut updated = Vec::new();
    for (sequence, change) in changes {
        if change.device_id != device && store.accept_for_retention(&change)? {
            updated.push(change.session.history.session.id.to_string());
        }
        received = sequence;
        store.acknowledge(base, sent, received)?;
    }
    Ok(updated)
}

pub async fn synchronize_definitions_once(
    client: &reqwest::Client,
    connection: &ServerConnection,
    store: &orchid_core::synchronization::definitions::DefinitionStore,
) -> Result<(), String> {
    let base = connection.url.trim_end_matches('/');
    orchid_core::skills::SkillCatalogue::refresh_origins(store)?;
    for definition in store.capture_owned()? {
        client
            .post(format!(
                "{base}/v1/definitions/{}",
                definition.master_device_id
            ))
            .bearer_auth(&connection.token)
            .json(&definition)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
    }
    let definitions: Vec<orchid_core::synchronization::definitions::Definition> = client
        .get(format!("{base}/v1/definitions"))
        .bearer_auth(&connection.token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    for definition in definitions {
        store.receive(&definition)?;
    }
    Ok(())
}
