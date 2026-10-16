use std::{net::SocketAddr, path::PathBuf};
pub struct Options {
    pub root: PathBuf,
    pub listen: SocketAddr,
    pub token: String,
    pub server: Option<String>,
}
impl Options {
    pub fn read(default_port: u16) -> Result<Self, String> {
        let mut root = None;
        let mut listen = format!("127.0.0.1:{default_port}");
        let mut server = None;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("Missing value for {arg}"))?;
            match arg.as_str() {
                "--data-dir" => root = Some(PathBuf::from(value)),
                "--listen" => listen = value,
                "--server" => server = Some(value),
                _ => return Err(format!("Unknown option {arg}")),
            }
        }
        let root =
            root.ok_or("Pass --data-dir with a separate absolute directory for this service")?;
        if !root.is_absolute() {
            return Err("--data-dir must be absolute".into());
        }
        let token = std::env::var("ORCHID_SERVICE_TOKEN")
            .map_err(|_| "Set ORCHID_SERVICE_TOKEN for authenticated service access")?;
        if token.trim().is_empty() {
            return Err("ORCHID_SERVICE_TOKEN cannot be empty".into());
        }
        Ok(Self {
            root,
            listen: listen
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            token,
            server,
        })
    }
}
