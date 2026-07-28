use super::catalog::{ReviewWorktreeCatalog, ReviewWorktreeOption};
use crate::worktree_runtime::{
    IsolatedTestRequest, TestActionOutcome, TestInstanceError, TestInstanceErrorKind,
    TestInstanceHandle, TestInstancePhase, TestInstanceStatus, TestSourceRef,
    WorktreeTestInstances,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceView {
    pub(crate) source_ref: String,
    pub(crate) label: String,
    pub(crate) revision: String,
}

impl From<&ReviewWorktreeOption> for ReviewSourceView {
    fn from(value: &ReviewWorktreeOption) -> Self {
        Self {
            source_ref: value.source_ref.clone(),
            label: value.label.clone(),
            revision: value.revision.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceView {
    pub(crate) instance_ref: String,
    pub(crate) name: String,
    pub(crate) source_label: String,
    pub(crate) phase: String,
    pub(crate) health: String,
    pub(crate) stale: bool,
    pub(crate) build: String,
    pub(crate) can_focus: bool,
}

#[derive(Clone)]
struct ReviewMetadata {
    name: String,
    source_label: String,
}

pub(crate) struct HumanReviewLauncherService {
    runtime: Arc<dyn WorktreeTestInstances>,
    catalog: Arc<ReviewWorktreeCatalog>,
    instances: Mutex<HashMap<String, ReviewMetadata>>,
    built: Mutex<HashSet<String>>,
    store: Mutex<Connection>,
}

impl HumanReviewLauncherService {
    pub(crate) fn new(
        runtime: Arc<dyn WorktreeTestInstances>,
        catalog: Arc<ReviewWorktreeCatalog>,
        store_path: &Path,
    ) -> Result<Self, String> {
        let store = Connection::open(store_path)
            .map_err(|error| format!("open review launcher state: {error}"))?;
        store
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS review_sessions (
                instance_ref TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_label TEXT NOT NULL,
                built INTEGER NOT NULL CHECK (built IN (0, 1))
            );",
            )
            .map_err(|error| format!("initialize review launcher state: {error}"))?;
        let (instances, built) = load_sessions(&store)?;
        Ok(Self {
            runtime,
            catalog,
            instances: Mutex::new(instances),
            built: Mutex::new(built),
            store: Mutex::new(store),
        })
    }

    pub(crate) fn sources(&self) -> Vec<ReviewSourceView> {
        self.catalog
            .options()
            .iter()
            .map(ReviewSourceView::from)
            .collect()
    }

    pub(crate) fn instances(&self) -> Vec<ReviewInstanceView> {
        let refs = self
            .instances
            .lock()
            .map(|instances| instances.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        refs.into_iter()
            .filter_map(|instance_ref| self.status(instance_ref).ok())
            .collect()
    }

    pub(crate) fn prepare(
        &self,
        source_ref: String,
        name: String,
    ) -> Result<ReviewInstanceView, String> {
        let source_label = self
            .catalog
            .label(&source_ref)
            .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
        let requested = self
            .runtime
            .request(
                IsolatedTestRequest::new(
                    TestSourceRef::new(source_ref).map_err(safe_error)?,
                    name.clone(),
                )
                .map_err(safe_error)?,
            )
            .map_err(safe_error)?;
        let instance_ref = requested.handle.opaque_ref().to_owned();
        self.instances
            .lock()
            .map_err(|_| "Review instance state is unavailable.".to_string())?
            .insert(
                instance_ref.clone(),
                ReviewMetadata {
                    name: name.clone(),
                    source_label: source_label.clone(),
                },
            );
        self.persist(&instance_ref, &name, &source_label, false)?;
        Ok(view(
            instance_ref,
            name,
            source_label,
            requested.status,
            "not-built",
        ))
    }

    pub(crate) fn build(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        let (handle, metadata) = self.resolve(&instance_ref)?;
        let result = self.runtime.build(&handle).map_err(safe_error)?;
        let build = match result.outcome {
            TestActionOutcome::Passed => {
                self.built
                    .lock()
                    .map_err(|_| "Review build state is unavailable.".to_string())?
                    .insert(instance_ref.clone());
                self.persist(&instance_ref, &metadata.name, &metadata.source_label, true)?;
                "passed"
            }
            TestActionOutcome::Failed => "failed",
        };
        Ok(view(
            instance_ref,
            metadata.name,
            metadata.source_label,
            result.status,
            build,
        ))
    }

    pub(crate) fn start(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        if !self
            .built
            .lock()
            .map_err(|_| "Review build state is unavailable.".to_string())?
            .contains(&instance_ref)
        {
            return Err("Build this review instance successfully before opening it.".into());
        }
        self.lifecycle(instance_ref, |runtime, handle| runtime.start(handle))
    }

    pub(crate) fn status(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.status(handle))
    }

    pub(crate) fn focus(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.focus(handle))
    }

    pub(crate) fn stop(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.stop(handle))
    }

    pub(crate) fn recover(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.recover(handle))
    }

    fn lifecycle(
        &self,
        instance_ref: String,
        operation: impl FnOnce(
            &dyn WorktreeTestInstances,
            &TestInstanceHandle,
        ) -> Result<TestInstanceStatus, TestInstanceError>,
    ) -> Result<ReviewInstanceView, String> {
        let (handle, metadata) = self.resolve(&instance_ref)?;
        let status = operation(self.runtime.as_ref(), &handle).map_err(safe_error)?;
        let build = if self
            .built
            .lock()
            .map_err(|_| "Review build state is unavailable.".to_string())?
            .contains(&instance_ref)
        {
            "passed"
        } else {
            "not-built"
        };
        Ok(view(
            instance_ref,
            metadata.name,
            metadata.source_label,
            status,
            build,
        ))
    }

    fn resolve(&self, instance_ref: &str) -> Result<(TestInstanceHandle, ReviewMetadata), String> {
        let metadata = self
            .instances
            .lock()
            .map_err(|_| "Review instance state is unavailable.".to_string())?
            .get(instance_ref)
            .map(|value| ReviewMetadata {
                name: value.name.clone(),
                source_label: value.source_label.clone(),
            })
            .ok_or_else(|| {
                "Prepare this review instance again in the current launcher session.".to_string()
            })?;
        Ok((
            TestInstanceHandle::from_opaque(instance_ref.to_owned()).map_err(safe_error)?,
            metadata,
        ))
    }

    fn persist(
        &self,
        instance_ref: &str,
        name: &str,
        source_label: &str,
        built: bool,
    ) -> Result<(), String> {
        self.store
            .lock()
            .map_err(|_| "Review launcher state is unavailable.".to_string())?
            .execute(
                "INSERT INTO review_sessions (instance_ref, name, source_label, built)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(instance_ref) DO UPDATE SET
                    name = excluded.name,
                    source_label = excluded.source_label,
                    built = excluded.built",
                params![instance_ref, name, source_label, i64::from(built)],
            )
            .map_err(|_| "Review launcher state could not be saved.".to_string())?;
        Ok(())
    }
}

fn load_sessions(
    store: &Connection,
) -> Result<(HashMap<String, ReviewMetadata>, HashSet<String>), String> {
    let mut statement = store
        .prepare("SELECT instance_ref, name, source_label, built FROM review_sessions")
        .map_err(|error| format!("read review launcher state: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ReviewMetadata {
                    name: row.get(1)?,
                    source_label: row.get(2)?,
                },
                row.get::<_, i64>(3)? != 0,
            ))
        })
        .map_err(|error| format!("read review launcher sessions: {error}"))?;
    let mut instances = HashMap::new();
    let mut built = HashSet::new();
    for row in rows {
        let (instance_ref, metadata, was_built) =
            row.map_err(|error| format!("decode review launcher session: {error}"))?;
        if was_built {
            built.insert(instance_ref.clone());
        }
        instances.insert(instance_ref, metadata);
    }
    Ok((instances, built))
}

fn view(
    instance_ref: String,
    name: String,
    source_label: String,
    status: TestInstanceStatus,
    build: &str,
) -> ReviewInstanceView {
    ReviewInstanceView {
        instance_ref,
        name,
        source_label,
        phase: phase(status.phase).into(),
        health: format!("{:?}", status.health).to_lowercase(),
        stale: status.stale,
        build: build.into(),
        can_focus: status.phase == TestInstancePhase::Running && !status.stale,
    }
}

fn phase(value: TestInstancePhase) -> &'static str {
    match value {
        TestInstancePhase::Prepared => "prepared",
        TestInstancePhase::Starting => "starting",
        TestInstancePhase::Running => "running",
        TestInstancePhase::Stopping => "stopping",
        TestInstancePhase::Stopped => "stopped",
        TestInstancePhase::Recovering => "recovering",
        TestInstancePhase::Recovered => "recovered",
    }
}

fn safe_error(error: TestInstanceError) -> String {
    match error.kind {
        TestInstanceErrorKind::InvalidRequest => "The review request is invalid.".into(),
        TestInstanceErrorKind::NotFound => "The review instance is no longer available.".into(),
        TestInstanceErrorKind::Unauthorized => {
            "Review instance ownership could not be verified.".into()
        }
        TestInstanceErrorKind::InvalidState => {
            "That action is not available in the current lifecycle state.".into()
        }
        TestInstanceErrorKind::OperationInProgress => {
            "Another lifecycle action is still in progress.".into()
        }
        TestInstanceErrorKind::Conflict => {
            "The selected source or review instance changed; prepare a fresh instance.".into()
        }
        TestInstanceErrorKind::Unavailable => {
            "The isolated review runtime could not complete this action. Check its private logs."
                .into()
        }
    }
}
