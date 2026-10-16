#[tokio::main]
async fn main() -> Result<(), String> {
    let options = orchid_services::options::Options::read(4480)?;
    let server = orchid_services::server::Server::open(&options.root, options.token)?;
    let listener = tokio::net::TcpListener::bind(options.listen)
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "Orchid synchronization server listening on {}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    axum::serve(listener, server.clone().router())
        .with_graceful_shutdown(async move {
            orchid_services::shutdown_signal().await;
            server.disconnect_hosts();
        })
        .await
        .map_err(|e| e.to_string())
}
