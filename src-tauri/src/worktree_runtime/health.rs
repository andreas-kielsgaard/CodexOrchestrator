use super::domain::{EndpointObservation, HealthObservation, InstanceProjection};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

pub(crate) trait HealthProbe: Send + Sync {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation;
}

pub(crate) struct TcpHealthProbe {
    timeout: Duration,
}

impl TcpHealthProbe {
    pub(crate) fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    fn endpoint(&self, port: u16) -> EndpointObservation {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        EndpointObservation {
            port,
            reachable: TcpStream::connect_timeout(&address, self.timeout).is_ok(),
        }
    }
}

impl Default for TcpHealthProbe {
    fn default() -> Self {
        Self::new(Duration::from_millis(100))
    }
}

impl HealthProbe for TcpHealthProbe {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        HealthObservation {
            vite: self.endpoint(projection.ports.vite),
            status: self.endpoint(projection.ports.status),
        }
    }
}
