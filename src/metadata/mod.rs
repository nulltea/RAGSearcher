pub mod models;

use anyhow::{Context, Result};
use models::{Paper, PaperCreate, PaperListParams, PaperStatus};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct MetadataStore {
    conn: Arc<Mutex<Connection>>,
}

impl MetadataStore {
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create metadata database directory")?;
        }

        let conn = Connection::open(db_path).context("Failed to open metadata database")?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .context("Failed to set SQLite pragmas")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS papers (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                authors TEXT NOT NULL DEFAULT '[]',
                source TEXT,
                published_date TEXT,
                paper_type TEXT NOT NULL DEFAULT 'research_paper',
                status TEXT NOT NULL DEFAULT 'processing',
                original_filename TEXT,
                chunk_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .context("Failed to create papers table")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn create_paper(&self, id: &str, create: PaperCreate) -> Result<Paper> {
        let conn = self.conn.clone();
        let id = id.to_string();
        let create = create.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();
            let authors_json = serde_json::to_string(&create.authors)?;

            conn.execute(
                "INSERT INTO papers (id, title, authors, source, published_date, paper_type, status, original_filename, chunk_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10)",
                rusqlite::params![
                    id,
                    create.title,
                    authors_json,
                    create.source,
                    create.published_date,
                    create.paper_type,
                    PaperStatus::Processing.to_string(),
                    create.original_filename,
                    now,
                    now,
                ],
            )?;

            Ok(Paper {
                id,
                title: create.title,
                authors: create.authors,
                source: create.source,
                published_date: create.published_date,
                paper_type: create.paper_type,
                status: PaperStatus::Processing,
                original_filename: create.original_filename,
                chunk_count: 0,
                created_at: now.clone(),
                updated_at: now,
            })
        })
        .await?
    }

    pub async fn get_paper(&self, id: &str) -> Result<Option<Paper>> {
        let conn = self.conn.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare(
                "SELECT id, title, authors, source, published_date, paper_type, status, original_filename, chunk_count, created_at, updated_at FROM papers WHERE id = ?1",
            )?;

            let paper = stmt.query_row(rusqlite::params![id], |row| {
                let authors_json: String = row.get(2)?;
                let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
                let status_str: String = row.get(6)?;

                Ok(Paper {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    authors,
                    source: row.get(3)?,
                    published_date: row.get(4)?,
                    paper_type: row.get(5)?,
                    status: PaperStatus::from_str(&status_str),
                    original_filename: row.get(7)?,
                    chunk_count: row.get::<_, i64>(8)? as usize,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            }).optional()?;

            Ok(paper)
        })
        .await?
    }

    pub async fn list_papers(&self, params: PaperListParams) -> Result<(Vec<Paper>, usize)> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let limit = params.limit.unwrap_or(20);
            let offset = params.offset.unwrap_or(0);

            let mut where_clauses = Vec::new();
            let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(ref status) = params.status {
                where_clauses.push(format!("status = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(status.clone()));
            }
            if let Some(ref paper_type) = params.paper_type {
                where_clauses.push(format!("paper_type = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(paper_type.clone()));
            }

            let where_sql = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };

            // Get total count
            let count_sql = format!("SELECT COUNT(*) FROM papers {}", where_sql);
            let total: usize = {
                let mut stmt = conn.prepare(&count_sql)?;
                let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
                stmt.query_row(refs.as_slice(), |row| row.get::<_, i64>(0))? as usize
            };

            // Get paginated results
            let query_sql = format!(
                "SELECT id, title, authors, source, published_date, paper_type, status, original_filename, chunk_count, created_at, updated_at FROM papers {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
                where_sql,
                bind_values.len() + 1,
                bind_values.len() + 2,
            );

            bind_values.push(Box::new(limit as i64));
            bind_values.push(Box::new(offset as i64));

            let mut stmt = conn.prepare(&query_sql)?;
            let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
            let papers = stmt
                .query_map(refs.as_slice(), |row| {
                    let authors_json: String = row.get(2)?;
                    let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
                    let status_str: String = row.get(6)?;

                    Ok(Paper {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        authors,
                        source: row.get(3)?,
                        published_date: row.get(4)?,
                        paper_type: row.get(5)?,
                        status: PaperStatus::from_str(&status_str),
                        original_filename: row.get(7)?,
                        chunk_count: row.get::<_, i64>(8)? as usize,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok((papers, total))
        })
        .await?
    }

    pub async fn update_paper_status(
        &self,
        id: &str,
        status: PaperStatus,
        chunk_count: usize,
    ) -> Result<()> {
        let conn = self.conn.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE papers SET status = ?1, chunk_count = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![status.to_string(), chunk_count as i64, now, id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_paper(&self, id: &str) -> Result<bool> {
        let conn = self.conn.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let rows = conn.execute("DELETE FROM papers WHERE id = ?1", rusqlite::params![id])?;
            Ok(rows > 0)
        })
        .await?
    }
}

use rusqlite::OptionalExtension;
