use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::path::PathBuf;
use std::str::FromStr;

pub use sqlx::Error as SqlError;

// ── ID types ──────────────────────────────────────────────────────────────────

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(ulid::Ulid::new().to_string())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

id_type!(SessionId);
id_type!(MessageId);
id_type!(PartId);
id_type!(ProjectId);

// ── Database ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn open(data_dir: &PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(data_dir)
            .await
            .context("create data dir")?;

        let db_path = data_dir.join("cc.db");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(opts)
            .await
            .context("open sqlite pool")?;

        // Run embedded migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("run migrations")?;

        Ok(Self { pool })
    }
}

// ── Session rows ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionRow {
    pub id: String,
    pub slug: String,
    pub project_id: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub parent_id: Option<String>,
    pub time_created: i64,
    pub time_updated: i64,
    pub summary_additions: Option<i64>,
    pub summary_deletions: Option<i64>,
    pub summary_files: Option<i64>,
}

pub struct SessionStore<'a> {
    pub db: &'a Db,
}

impl<'a> SessionStore<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub async fn insert(&self, row: &SessionRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions
                (id, slug, project_id, directory, title, version, parent_id, time_created, time_updated)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.id)
        .bind(&row.slug)
        .bind(&row.project_id)
        .bind(&row.directory)
        .bind(&row.title)
        .bind(&row.version)
        .bind(&row.parent_id)
        .bind(row.time_created)
        .bind(row.time_updated)
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<SessionRow>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT * FROM sessions ORDER BY time_updated DESC",
        )
        .fetch_all(&self.db.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get(&self, id: &str) -> Result<Option<SessionRow>> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?;
        Ok(row)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    pub async fn update_title(&self, id: &str, title: &str) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        sqlx::query("UPDATE sessions SET title = ?, time_updated = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(id)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }
}

// ── Message rows ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub time_created: i64,
}

pub struct MessageStore<'a> {
    pub db: &'a Db,
}

impl<'a> MessageStore<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub async fn insert(&self, row: &MessageRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO messages (id, session_id, role, time_created) VALUES (?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.session_id)
        .bind(&row.role)
        .bind(row.time_created)
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    pub async fn list_for_session(&self, session_id: &str) -> Result<Vec<MessageRow>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY time_created ASC",
        )
        .bind(session_id)
        .fetch_all(&self.db.pool)
        .await?;
        Ok(rows)
    }
}

// ── Part rows ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PartRow {
    pub id: String,
    pub message_id: String,
    pub kind: String,
    pub content: String, // JSON blob
    pub order_idx: i64,
}

pub struct PartStore<'a> {
    pub db: &'a Db,
}

impl<'a> PartStore<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub async fn insert(&self, row: &PartRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO parts (id, message_id, kind, content, order_idx) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.message_id)
        .bind(&row.kind)
        .bind(&row.content)
        .bind(row.order_idx)
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    pub async fn list_for_message(&self, message_id: &str) -> Result<Vec<PartRow>> {
        let rows = sqlx::query_as::<_, PartRow>(
            "SELECT * FROM parts WHERE message_id = ? ORDER BY order_idx ASC",
        )
        .bind(message_id)
        .fetch_all(&self.db.pool)
        .await?;
        Ok(rows)
    }
}
