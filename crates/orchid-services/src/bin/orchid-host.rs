use std::sync::Arc;
fn main() -> Result<(), String> {
    if orchid_core::harness_engine::sidecar::run_if_requested() {
        return Ok(());
    }
    let options = orchid_services::options::Options::read(4481)?;
    let host = Arc::new(orchid_core::composition::host::DeviceHost::open(
        &options.root,
    )?);
    let id = orchid_core::synchronization::repository::ReplicaStore::new(host.database.clone())
        .device_id()?;
    println!("Orchid execution host {id}");
    tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())?
        .block_on(async {
            let handler: Arc<orchid_services::host::CommandHandler> = {
                let host = host.clone();
                Arc::new(move |command| {
                    if command.method == "local.registerProvider" {
                        return host.register_provider(
                            command.input["provider"]
                                .as_str()
                                .ok_or("Missing provider")?,
                            command.input["home"]
                                .as_str()
                                .ok_or("Missing provider home")?,
                            command.input["executable"].as_str().map(str::to_owned),
                        );
                    }
                    orchid_core::agent_sessions::client::dispatch_on_device(
                        &id,
                        &host.sessions.application,
                        &host.repository,
                        command,
                    )
                })
            };
            let api = orchid_services::host_api::HostApi {
                token: options.token.clone(),
                handler,
                queries: host.sessions.application.queries().with_replicas(
                    orchid_core::synchronization::repository::ReplicaStore::new(
                        host.database.clone(),
                    ),
                ),
            };
            let listener = tokio::net::TcpListener::bind(options.listen)
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "Local device API listening on {}",
                listener.local_addr().map_err(|e| e.to_string())?
            );
            let server = tokio::spawn(async move { axum::serve(listener, api.router()).await });
            if let Some(url) = options.server {
                tokio::spawn(orchid_services::host::run_connections(
                    host.database.clone(),
                    host.repository.clone(),
                    host.sessions.application.clone(),
                    orchid_core::synchronization::ServerConnection {
                        url,
                        token: options.token,
                    },
                ));
            }
            orchid_services::shutdown_signal().await;
            server.abort();
            host.shutdown()
        })
}
