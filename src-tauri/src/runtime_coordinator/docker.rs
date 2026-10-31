use std::time::Duration;
use tokio::process::Command;

async fn output(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("docker");
    command.args(args).kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let result = tokio::time::timeout(Duration::from_secs(10), command.output())
        .await
        .map_err(|_| "Docker did not respond. Open Docker Desktop and retry.")?
        .map_err(|_| "Docker is unavailable. Install or open Docker Desktop and retry.")?;
    if !result.status.success() {
        return Err("Cannot access the Docker coordinator. Open Docker Desktop and start the coordinator deployment, then retry.".into());
    }
    if result.stdout.len() > 16_384 {
        return Err("Docker returned an unexpected reply".into());
    }
    String::from_utf8(result.stdout).map_err(|_| "Docker returned an invalid reply".into())
}

pub(super) async fn connection() -> Result<(String, String), String> {
    let containers = output(&[
        "ps",
        "--filter",
        "label=com.docker.compose.project=orchid-coordinator",
        "--filter",
        "label=com.docker.compose.service=coordinator",
        "--format",
        "{{.ID}}",
    ])
    .await?;
    let ids: Vec<_> = containers.split_whitespace().collect();
    if ids.len() != 1 || !ids[0].bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Start one Orchid coordinator deployment in Docker, then retry. Use the runtime-coordinator Compose deployment with project name orchid-coordinator.".into());
    }
    let ports = output(&["port", ids[0], "41800/tcp"]).await?;
    let url = loopback_endpoint(&ports)?;
    let token = output(&["exec", ids[0], "cat", "/run/secrets/coordinator_token"]).await?;
    if token.trim().len() < 32 {
        return Err("The Docker coordinator access token is invalid".into());
    }
    Ok((url, token.trim().into()))
}

fn loopback_endpoint(ports: &str) -> Result<String, String> {
    let bindings: Vec<_> = ports
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if bindings.len() != 1 {
        return Err("The Docker coordinator must publish one loopback port".into());
    }
    let port = bindings[0]
        .strip_prefix("127.0.0.1:")
        .and_then(|p| p.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .ok_or("The Docker coordinator must be published on 127.0.0.1")?;
    Ok(format!("http://127.0.0.1:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_credentials_are_only_used_for_one_loopback_binding() {
        assert_eq!(
            loopback_endpoint("127.0.0.1:41800\n").unwrap(),
            "http://127.0.0.1:41800"
        );
        assert!(loopback_endpoint("0.0.0.0:41800").is_err());
        assert!(loopback_endpoint("127.0.0.1:41800\n127.0.0.1:41801").is_err());
        assert!(loopback_endpoint("127.0.0.1:0").is_err());
    }
}
