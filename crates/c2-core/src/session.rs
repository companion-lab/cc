use c2_storage::{SessionId, ProjectId, Db, SessionRow, SessionStore};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub directory: String,
    pub title: String,
    pub parent_id: Option<SessionId>,
    pub time_created: i64,
    pub time_updated: i64,
}

impl Session {
    pub fn new(directory: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: SessionId::new(),
            project_id: ProjectId::new(),
            directory: directory.into(),
            title: title.into(),
            parent_id: None,
            time_created: now,
            time_updated: now,
        }
    }

    pub async fn save(&self, db: &Db) -> Result<()> {
        let store = SessionStore::new(db);
        let row = SessionRow {
            id: self.id.to_string(),
            slug: slug_from_title(&self.title),
            project_id: self.project_id.to_string(),
            directory: self.directory.clone(),
            title: self.title.clone(),
            version: "1".to_string(),
            parent_id: self.parent_id.as_ref().map(|p| p.to_string()),
            time_created: self.time_created,
            time_updated: self.time_updated,
            summary_additions: None,
            summary_deletions: None,
            summary_files: None,
        };
        store.insert(&row).await
    }

    pub async fn list(db: &Db) -> Result<Vec<Session>> {
        let store = SessionStore::new(db);
        let rows = store.list().await?;
        Ok(rows.into_iter().map(Session::from).collect())
    }

    pub async fn get(db: &Db, id: &str) -> Result<Option<Session>> {
        let store = SessionStore::new(db);
        Ok(store.get(id).await?.map(Session::from))
    }

    pub async fn delete(db: &Db, id: &str) -> Result<()> {
        SessionStore::new(db).delete(id).await
    }
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            id: SessionId(row.id),
            project_id: ProjectId(row.project_id),
            directory: row.directory,
            title: row.title,
            parent_id: row.parent_id.map(SessionId),
            time_created: row.time_created,
            time_updated: row.time_updated,
        }
    }
}

fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
