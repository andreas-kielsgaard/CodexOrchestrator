//! Cache policy for semantic agent-access capability evidence.
//!
//! Concrete adapters own probing and translation. This module only decides when a fresh snapshot
//! can be reused and provides explicit refresh and invalidation operations.

use crate::agent_sessions::ports::{
    AgentAccessCapabilityDiscovery, AgentAccessCapabilitySnapshot, CapabilityRefresh,
};
use chrono::{DateTime, Utc};
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct AgentAccessCapabilityCache {
    snapshot: Mutex<Option<AgentAccessCapabilitySnapshot>>,
}

impl AgentAccessCapabilityCache {
    pub(crate) fn resolve(
        &self,
        refresh: CapabilityRefresh,
        now: DateTime<Utc>,
        discovery: &dyn AgentAccessCapabilityDiscovery,
    ) -> AgentAccessCapabilitySnapshot {
        let mut cached = self.lock();
        if refresh == CapabilityRefresh::UseFreshCache {
            if let Some(snapshot) = cached
                .as_ref()
                .filter(|snapshot| snapshot.is_fresh_at(now))
                .cloned()
            {
                return snapshot;
            }
        }

        let snapshot = discovery.discover_capabilities(now);
        *cached = Some(snapshot.clone());
        snapshot
    }

    pub(crate) fn snapshot(&self) -> Option<AgentAccessCapabilitySnapshot> {
        self.lock().clone()
    }

    pub(crate) fn invalidate(&self) {
        *self.lock() = None;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<AgentAccessCapabilitySnapshot>> {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::ports::{
        AgentAccessCapabilities, CapabilityDiscoveryState, CapabilityProvenance, CapabilitySupport,
        InvocationCapabilities,
    };
    use chrono::Duration;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Barrier,
        },
        thread,
        time::Duration as StdDuration,
    };

    struct Discovery {
        count: AtomicUsize,
    }

    impl Discovery {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }
    }

    impl AgentAccessCapabilityDiscovery for Discovery {
        fn discover_capabilities(
            &self,
            observed_at: DateTime<Utc>,
        ) -> AgentAccessCapabilitySnapshot {
            self.count.fetch_add(1, Ordering::SeqCst);
            AgentAccessCapabilitySnapshot {
                capabilities: AgentAccessCapabilities {
                    start: InvocationCapabilities {
                        structured_events: CapabilitySupport::Supported,
                        model_selection: CapabilitySupport::Unsupported,
                        sandbox_selection: CapabilitySupport::Unknown,
                    },
                    resume: InvocationCapabilities::default(),
                },
                discovery_state: CapabilityDiscoveryState::Observed,
                provenance: CapabilityProvenance {
                    source: "test_probe".to_string(),
                    runtime_version: Some("test-1".to_string()),
                },
                observed_at,
                valid_until: observed_at + Duration::minutes(30),
                unavailable_reason: None,
            }
        }
    }

    #[test]
    fn reuses_fresh_evidence_and_refreshes_expired_evidence() {
        let cache = AgentAccessCapabilityCache::default();
        let discovery = Discovery::new();
        let first_at = "2026-07-15T10:00:00Z".parse().expect("timestamp");

        let first = cache.resolve(CapabilityRefresh::UseFreshCache, first_at, &discovery);
        let cached = cache.resolve(
            CapabilityRefresh::UseFreshCache,
            first_at + Duration::minutes(10),
            &discovery,
        );
        let expired = cache.resolve(
            CapabilityRefresh::UseFreshCache,
            first_at + Duration::minutes(31),
            &discovery,
        );

        assert_eq!(first, cached);
        assert_ne!(first.observed_at, expired.observed_at);
        assert_eq!(discovery.count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn explicit_refresh_and_invalidation_bypass_cached_evidence() {
        let cache = AgentAccessCapabilityCache::default();
        let discovery = Discovery::new();
        let first_at = "2026-07-15T10:00:00Z".parse().expect("timestamp");
        cache.resolve(CapabilityRefresh::UseFreshCache, first_at, &discovery);

        cache.resolve(
            CapabilityRefresh::Refresh,
            first_at + Duration::minutes(1),
            &discovery,
        );
        cache.invalidate();
        cache.resolve(
            CapabilityRefresh::UseFreshCache,
            first_at + Duration::minutes(2),
            &discovery,
        );

        assert_eq!(discovery.count.load(Ordering::SeqCst), 3);
    }

    struct UnavailableDiscovery;

    impl AgentAccessCapabilityDiscovery for UnavailableDiscovery {
        fn discover_capabilities(
            &self,
            observed_at: DateTime<Utc>,
        ) -> AgentAccessCapabilitySnapshot {
            AgentAccessCapabilitySnapshot {
                capabilities: AgentAccessCapabilities::default(),
                discovery_state: CapabilityDiscoveryState::Unavailable,
                provenance: CapabilityProvenance {
                    source: "test_probe".to_string(),
                    runtime_version: None,
                },
                observed_at,
                valid_until: observed_at + Duration::minutes(1),
                unavailable_reason: Some("probe executable was unavailable".to_string()),
            }
        }
    }

    #[test]
    fn unavailable_discovery_remains_unknown_and_cacheable() {
        let cache = AgentAccessCapabilityCache::default();
        let observed_at = "2026-07-15T10:00:00Z".parse().expect("timestamp");

        let snapshot = cache.resolve(
            CapabilityRefresh::UseFreshCache,
            observed_at,
            &UnavailableDiscovery,
        );

        assert_eq!(
            snapshot.discovery_state,
            CapabilityDiscoveryState::Unavailable
        );
        assert_eq!(
            snapshot.capabilities.start.model_selection,
            CapabilitySupport::Unknown
        );
        assert_eq!(cache.snapshot(), Some(snapshot));
    }

    struct SlowDiscovery {
        count: AtomicUsize,
    }

    impl AgentAccessCapabilityDiscovery for SlowDiscovery {
        fn discover_capabilities(
            &self,
            observed_at: DateTime<Utc>,
        ) -> AgentAccessCapabilitySnapshot {
            self.count.fetch_add(1, Ordering::SeqCst);
            thread::sleep(StdDuration::from_millis(20));
            Discovery::new().discover_capabilities(observed_at)
        }
    }

    #[test]
    fn concurrent_fresh_resolution_serializes_one_discovery() {
        let cache = Arc::new(AgentAccessCapabilityCache::default());
        let discovery = Arc::new(SlowDiscovery {
            count: AtomicUsize::new(0),
        });
        let barrier = Arc::new(Barrier::new(5));
        let observed_at = "2026-07-15T10:00:00Z".parse().expect("timestamp");
        let workers = (0..4)
            .map(|_| {
                let cache = cache.clone();
                let discovery = discovery.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    cache.resolve(
                        CapabilityRefresh::UseFreshCache,
                        observed_at,
                        discovery.as_ref(),
                    )
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let snapshots = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker"))
            .collect::<Vec<_>>();

        assert!(snapshots.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(discovery.count.load(Ordering::SeqCst), 1);
    }
}
