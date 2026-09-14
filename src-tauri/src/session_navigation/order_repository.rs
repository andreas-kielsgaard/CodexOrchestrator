use super::order::{NavigationOrder, NavigationOrderScope};
use crate::persistence::{ActiveDatabase, ManagedOperationError};
use std::sync::Arc;

pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS session_navigation_order (
    scope TEXT PRIMARY KEY NOT NULL,
    ordered_ids TEXT NOT NULL
);
"#;

pub(crate) struct NavigationOrderRepository(Arc<ActiveDatabase>);
impl NavigationOrderRepository {
    pub(crate) fn new(database: Arc<ActiveDatabase>) -> Self {
        Self(database)
    }

    pub(crate) fn load(&self) -> Result<Vec<NavigationOrder>, String> {
        self.0
            .read("load navigation order", |connection| -> Result<_, String> {
                let mut statement = connection
                    .prepare(
                        "SELECT scope, ordered_ids FROM session_navigation_order ORDER BY scope",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?;
                rows.map(|row| {
                    let (scope, ids) = row.map_err(|e| e.to_string())?;
                    Ok(NavigationOrder {
                        scope: serde_json::from_str(&scope).map_err(|e| e.to_string())?,
                        ordered_ids: serde_json::from_str(&ids).map_err(|e| e.to_string())?,
                    })
                })
                .collect()
            })
            .map_err(ManagedOperationError::into_string)
    }

    pub(crate) fn save(&self, scope: &NavigationOrderScope, ids: &[String]) -> Result<(), String> {
        self.save_with_placement(scope, ids, None)
    }
    pub(crate) fn save_with_placement(
        &self,
        scope: &NavigationOrderScope,
        ids: &[String],
        placement: Option<(
            &crate::agent_sessions::domain::AgentSessionId,
            &crate::agent_sessions::organization::SessionPlacement,
        )>,
    ) -> Result<(), String> {
        let scope = serde_json::to_string(scope).map_err(|e| e.to_string())?;
        let ids = serde_json::to_string(ids).map_err(|e| e.to_string())?;
        self.0.write("save navigation order", |transaction| {
            if let Some((id, placement)) = placement { crate::agent_sessions::repository::write_placement(transaction, id, placement).map_err(|e| e.to_string())?; }
            transaction.execute("INSERT INTO session_navigation_order(scope, ordered_ids) VALUES(?1, ?2) ON CONFLICT(scope) DO UPDATE SET ordered_ids=excluded.ordered_ids", rusqlite::params![scope, ids]).map(|_| ()).map_err(|e| e.to_string())
        }).map_err(ManagedOperationError::into_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orders_replace_one_scope_and_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("navigation.sqlite");
        let repo = NavigationOrderRepository::new(crate::product_database::open(&path).unwrap());
        let sections = NavigationOrderScope::Sections {
            repository_id: "a".into(),
        };
        repo.save(
            &NavigationOrderScope::Repositories,
            &["a".into(), "b".into()],
        )
        .unwrap();
        repo.save(&sections, &["workflows".into(), "sessions".into()])
            .unwrap();
        repo.save(
            &NavigationOrderScope::Repositories,
            &["b".into(), "a".into()],
        )
        .unwrap();
        repo.save(&NavigationOrderScope::Pinned, &["s2".into(), "s1".into()])
            .unwrap();
        repo.save(
            &NavigationOrderScope::Sessions {
                folder_id: "unfiled".into(),
            },
            &["s1".into(), "s2".into()],
        )
        .unwrap();
        drop(repo);
        let repo = NavigationOrderRepository::new(crate::product_database::open(&path).unwrap());
        let orders = repo.load().unwrap();
        assert_eq!(orders.len(), 4);
        assert_eq!(
            orders
                .iter()
                .find(|o| o.scope == NavigationOrderScope::Pinned)
                .unwrap()
                .ordered_ids,
            ["s2", "s1"]
        );
        assert_eq!(
            orders
                .iter()
                .find(|o| o.scope
                    == (NavigationOrderScope::Sessions {
                        folder_id: "unfiled".into()
                    }))
                .unwrap()
                .ordered_ids,
            ["s1", "s2"]
        );
        assert_eq!(
            orders
                .iter()
                .find(|o| o.scope == NavigationOrderScope::Repositories)
                .unwrap()
                .ordered_ids,
            ["b", "a"]
        );
        assert_eq!(
            orders
                .iter()
                .find(|o| o.scope == sections)
                .unwrap()
                .ordered_ids,
            ["workflows", "sessions"]
        );
    }
}
