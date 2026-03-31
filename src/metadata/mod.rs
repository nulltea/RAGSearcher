pub mod algorithms;
pub mod models;

use anyhow::{Context, Result};
use models::{Paper, PaperCreate, PaperListParams, PaperStatus, Pattern, PatternStatus};
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
                file_path TEXT,
                chunk_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                claim TEXT,
                evidence TEXT,
                context TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                confidence TEXT NOT NULL DEFAULT 'medium',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS algorithms (
                id TEXT PRIMARY KEY,
                paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                description TEXT,
                steps TEXT NOT NULL DEFAULT '[]',
                inputs TEXT NOT NULL DEFAULT '[]',
                outputs TEXT NOT NULL DEFAULT '[]',
                preconditions TEXT NOT NULL DEFAULT '[]',
                complexity TEXT,
                mathematical_notation TEXT,
                pseudocode TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                evidence_ids TEXT NOT NULL DEFAULT '[]',
                confidence TEXT NOT NULL DEFAULT 'medium',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .context("Failed to create tables")?;

        // Migrations for existing databases
        let _ = conn.execute_batch(
            "ALTER TABLE papers ADD COLUMN file_path TEXT;",
        );

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
                "INSERT INTO papers (id, title, authors, source, published_date, paper_type, status, original_filename, file_path, chunk_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11)",
                rusqlite::params![
                    id,
                    create.title,
                    authors_json,
                    create.source,
                    create.published_date,
                    create.paper_type,
                    PaperStatus::Processing.to_string(),
                    create.original_filename,
                    create.file_path,
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
                file_path: create.file_path,
                chunk_count: 0,
                pattern_count: 0,
                algorithm_count: 0,
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
                "SELECT p.id, p.title, p.authors, p.source, p.published_date, p.paper_type, p.status, p.original_filename, p.file_path, p.chunk_count, p.created_at, p.updated_at,
                 (SELECT COUNT(*) FROM patterns WHERE paper_id = p.id) as pattern_count,
                 (SELECT COUNT(*) FROM algorithms WHERE paper_id = p.id) as algorithm_count
                 FROM papers p WHERE p.id = ?1",
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
                    file_path: row.get(8)?,
                    chunk_count: row.get::<_, i64>(9)? as usize,
                    pattern_count: row.get::<_, i64>(12)? as usize,
                    algorithm_count: row.get::<_, i64>(13)? as usize,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
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
                where_clauses.push(format!("p.status = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(status.clone()));
            }
            if let Some(ref paper_type) = params.paper_type {
                where_clauses.push(format!("p.paper_type = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(paper_type.clone()));
            }

            let where_sql = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };

            // Get total count
            let count_sql = format!("SELECT COUNT(*) FROM papers p {}", where_sql);
            let total: usize = {
                let mut stmt = conn.prepare(&count_sql)?;
                let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
                stmt.query_row(refs.as_slice(), |row| row.get::<_, i64>(0))? as usize
            };

            // Get paginated results
            let query_sql = format!(
                "SELECT p.id, p.title, p.authors, p.source, p.published_date, p.paper_type, p.status, p.original_filename, p.file_path, p.chunk_count, p.created_at, p.updated_at,
                 (SELECT COUNT(*) FROM patterns WHERE paper_id = p.id) as pattern_count,
                 (SELECT COUNT(*) FROM algorithms WHERE paper_id = p.id) as algorithm_count
                 FROM papers p {} ORDER BY p.created_at DESC LIMIT ?{} OFFSET ?{}",
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
                        file_path: row.get(8)?,
                        chunk_count: row.get::<_, i64>(9)? as usize,
                        pattern_count: row.get::<_, i64>(12)? as usize,
                        algorithm_count: row.get::<_, i64>(13)? as usize,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
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

    // --- Pattern CRUD ---

    pub async fn create_pattern(
        &self,
        paper_id: &str,
        name: &str,
        claim: Option<&str>,
        evidence: Option<&str>,
        context: Option<&str>,
        tags: &[String],
        confidence: &str,
    ) -> Result<Pattern> {
        let conn = self.conn.clone();
        let id = uuid::Uuid::new_v4().to_string();
        let paper_id = paper_id.to_string();
        let name = name.to_string();
        let claim = claim.map(|s| s.to_string());
        let evidence = evidence.map(|s| s.to_string());
        let context = context.map(|s| s.to_string());
        let tags_json = serde_json::to_string(tags)?;
        let confidence = confidence.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO patterns (id, paper_id, name, claim, evidence, context, tags, confidence, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    id, paper_id, name, claim, evidence, context, tags_json, confidence,
                    PatternStatus::Pending.to_string(), now, now,
                ],
            )?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(Pattern {
                id,
                paper_id,
                name,
                claim,
                evidence,
                context,
                tags,
                confidence,
                status: PatternStatus::Pending,
                created_at: now.clone(),
                updated_at: now,
            })
        })
        .await?
    }

    pub async fn list_patterns(
        &self,
        paper_id: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<Pattern>> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();
        let status_filter = status_filter.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
                if let Some(ref status) = status_filter {
                    (
                        "SELECT id, paper_id, name, claim, evidence, context, tags, confidence, status, created_at, updated_at FROM patterns WHERE paper_id = ?1 AND status = ?2 ORDER BY created_at".to_string(),
                        vec![Box::new(paper_id), Box::new(status.clone())],
                    )
                } else {
                    (
                        "SELECT id, paper_id, name, claim, evidence, context, tags, confidence, status, created_at, updated_at FROM patterns WHERE paper_id = ?1 ORDER BY created_at".to_string(),
                        vec![Box::new(paper_id)],
                    )
                };

            let mut stmt = conn.prepare(&sql)?;
            let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let patterns = stmt
                .query_map(refs.as_slice(), |row| {
                    let tags_json: String = row.get(6)?;
                    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                    let status_str: String = row.get(8)?;
                    Ok(Pattern {
                        id: row.get(0)?,
                        paper_id: row.get(1)?,
                        name: row.get(2)?,
                        claim: row.get(3)?,
                        evidence: row.get(4)?,
                        context: row.get(5)?,
                        tags,
                        confidence: row.get(7)?,
                        status: PatternStatus::from_str(&status_str),
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(patterns)
        })
        .await?
    }

    pub async fn update_pattern_status(&self, id: &str, status: PatternStatus) -> Result<()> {
        let conn = self.conn.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE patterns SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status.to_string(), now, id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_patterns_by_paper(&self, paper_id: &str) -> Result<()> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            conn.execute(
                "DELETE FROM patterns WHERE paper_id = ?1",
                rusqlite::params![paper_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn search_papers(
        &self,
        query: Option<&str>,
        status: Option<&str>,
        paper_type: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Paper>, usize)> {
        let conn = self.conn.clone();
        let query = query.and_then(|q| if q.trim().is_empty() { None } else { Some(format!("%{}%", q)) });
        let status = status.map(|s| s.to_string());
        let paper_type = paper_type.map(|t| t.to_string());

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let mut where_clauses = Vec::new();
            let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(ref q) = query {
                let i = bind_values.len() + 1;
                where_clauses.push(format!("(p.title LIKE ?{} OR p.authors LIKE ?{})", i, i + 1));
                bind_values.push(Box::new(q.clone()));
                bind_values.push(Box::new(q.clone()));
            }
            if let Some(ref s) = status {
                where_clauses.push(format!("p.status = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(s.clone()));
            }
            if let Some(ref pt) = paper_type {
                where_clauses.push(format!("p.paper_type = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(pt.clone()));
            }

            let where_sql = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };

            let count_sql = format!("SELECT COUNT(*) FROM papers p {}", where_sql);
            let total: usize = {
                let mut stmt = conn.prepare(&count_sql)?;
                let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
                stmt.query_row(refs.as_slice(), |row| row.get::<_, i64>(0))? as usize
            };

            let query_sql = format!(
                "SELECT p.id, p.title, p.authors, p.source, p.published_date, p.paper_type, p.status, p.original_filename, p.file_path, p.chunk_count, p.created_at, p.updated_at,
                 (SELECT COUNT(*) FROM patterns WHERE paper_id = p.id) as pattern_count,
                 (SELECT COUNT(*) FROM algorithms WHERE paper_id = p.id) as algorithm_count
                 FROM papers p {} ORDER BY p.created_at DESC LIMIT ?{} OFFSET ?{}",
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
                        file_path: row.get(8)?,
                        chunk_count: row.get::<_, i64>(9)? as usize,
                        pattern_count: row.get::<_, i64>(12)? as usize,
                        algorithm_count: row.get::<_, i64>(13)? as usize,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok((papers, total))
        })
        .await?
    }

    pub async fn count_patterns_by_status(
        &self,
        paper_id: &str,
    ) -> Result<(usize, usize, usize)> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM patterns WHERE paper_id = ?1 GROUP BY status",
            )?;
            let mut pending = 0usize;
            let mut approved = 0usize;
            let mut rejected = 0usize;

            let rows = stmt.query_map(rusqlite::params![paper_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?;

            for row in rows {
                let (status, count) = row?;
                match status.as_str() {
                    "pending" => pending = count,
                    "approved" => approved = count,
                    "rejected" => rejected = count,
                    _ => {}
                }
            }

            Ok((pending, approved, rejected))
        })
        .await?
    }
}

use rusqlite::OptionalExtension;
