use fs2::FileExt;
use orchid_coordinator::{http, service::Service};
use std::{fs, path::PathBuf};
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("orchid-coordinator: {e}");
        std::process::exit(1)
    }
}
async fn run() -> Result<(), String> {
    let mut root = PathBuf::from("coordinator-data");
    let mut bind = "127.0.0.1:0".to_string();
    let mut token_file = None;
    let mut ready_file = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--data" => root = PathBuf::from(args.next().ok_or("Missing data directory")?),
            "--bind" => bind = args.next().ok_or("Missing bind address")?,
            "--token-file" => {
                token_file = Some(PathBuf::from(args.next().ok_or("Missing token file")?))
            }
            "--ready-file" => {
                ready_file = Some(PathBuf::from(args.next().ok_or("Missing ready file")?))
            }
            "--help" => {
                println!("orchid-coordinator --data <directory> --bind <address> --token-file <file> [--ready-file <file>]");
                return Ok(());
            }
            _ => return Err("Unknown coordinator option".into()),
        }
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "A coordinator already owns this data directory")?;
    let token = fs::read_to_string(token_file.ok_or("--token-file is required")?)
        .map_err(|_| "Cannot read coordinator access token")?;
    let token = token.trim().to_string();
    if token.len() < 32 {
        return Err("Coordinator access token must contain at least 32 characters".into());
    }
    let service = Service::open(&root, "https://api.hetzner.cloud/v1")?;
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    if let Some(path) = ready_file {
        fs::write(path,serde_json::to_vec(&serde_json::json!({"url":format!("http://{addr}"),"coordinatorId":service.health()?.coordinator_id})).unwrap()).map_err(|e|e.to_string())?
    }
    println!("Coordinator listening on {addr}");
    axum::serve(listener, http::router(service, token))
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| e.to_string())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("termination signal");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
