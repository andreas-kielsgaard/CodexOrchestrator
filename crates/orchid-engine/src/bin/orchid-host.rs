use orchid_engine::host::{connect, Host, HostConfiguration, JsonLineOutput};
use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("connect") {
        return Err("Usage: orchid-host connect [--config <path>]".into());
    }
    let config_path = match args.next().as_deref() {
        Some("--config") => PathBuf::from(args.next().ok_or("Missing configuration path")?),
        None => PathBuf::from(
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .ok_or("Home directory unavailable")?,
        )
        .join(".codex-orchestrator/host/config.json"),
        _ => return Err("Usage: orchid-host connect [--config <path>]".into()),
    };
    let configuration: HostConfiguration = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    let host = Arc::new(Host::new(
        configuration,
        config_path
            .parent()
            .ok_or("Configuration parent unavailable")?
            .join("sessions"),
    )?);
    let output = Arc::new(JsonLineOutput(Mutex::new(io::stdout())));
    let result = connect(host.clone(), io::stdin().lock(), output);
    let shutdown = host.shutdown();
    result.and(shutdown)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("orchid-host: {error}");
        std::process::exit(1);
    }
}
