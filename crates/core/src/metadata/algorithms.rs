use anyhow::Result;

use super::MetadataStore;
use super::models::{Algorithm, AlgorithmIORow, AlgorithmStepRow, PatternStatus};

impl MetadataStore {
    pub async fn create_algorithm(
        &self,
        paper_id: &str,
        name: &str,
        description: Option<&str>,
        steps: &[AlgorithmStepRow],
        inputs: &[AlgorithmIORow],
        outputs: &[AlgorithmIORow],
        preconditions: &[String],
        complexity: Option<&str>,
        mathematical_notation: Option<&str>,
        pseudocode: Option<&str>,
        tags: &[String],
        evidence_ids: &[String],
        confidence: &str,
    ) -> Result<Algorithm> {
        let conn = self.conn.clone();
        let id = uuid::Uuid::new_v4().to_string();
        let paper_id = paper_id.to_string();
        let name = name.to_string();
        let description = description.map(|s| s.to_string());
        let steps_json = serde_json::to_string(steps)?;
        let inputs_json = serde_json::to_string(inputs)?;
        let outputs_json = serde_json::to_string(outputs)?;
        let preconditions_json = serde_json::to_string(preconditions)?;
        let complexity = complexity.map(|s| s.to_string());
        let mathematical_notation = mathematical_notation.map(|s| s.to_string());
        let pseudocode = pseudocode.map(|s| s.to_string());
        let tags_json = serde_json::to_string(tags)?;
        let evidence_ids_json = serde_json::to_string(evidence_ids)?;
        let confidence = confidence.to_string();

        let steps_clone = steps.to_vec();
        let inputs_clone = inputs.to_vec();
        let outputs_clone = outputs.to_vec();
        let preconditions_clone = preconditions.to_vec();
        let tags_clone = tags.to_vec();
        let evidence_ids_clone = evidence_ids.to_vec();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO algorithms (id, paper_id, name, description, steps, inputs, outputs, preconditions, complexity, mathematical_notation, pseudocode, tags, evidence_ids, confidence, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                rusqlite::params![
                    id, paper_id, name, description,
                    steps_json, inputs_json, outputs_json, preconditions_json,
                    complexity, mathematical_notation, pseudocode,
                    tags_json, evidence_ids_json, confidence,
                    PatternStatus::Pending.to_string(), now, now,
                ],
            )?;

            Ok(Algorithm {
                id,
                paper_id,
                name,
                description,
                steps: steps_clone,
                inputs: inputs_clone,
                outputs: outputs_clone,
                preconditions: preconditions_clone,
                complexity,
                mathematical_notation,
                pseudocode,
                tags: tags_clone,
                evidence_ids: evidence_ids_clone,
                confidence,
                status: PatternStatus::Pending,
                created_at: now.clone(),
                updated_at: now,
            })
        })
        .await?
    }

    pub async fn list_algorithms(
        &self,
        paper_id: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<Algorithm>> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();
        let status_filter = status_filter.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
                if let Some(ref status) = status_filter {
                    (
                        "SELECT id, paper_id, name, description, steps, inputs, outputs, preconditions, complexity, mathematical_notation, pseudocode, tags, evidence_ids, confidence, status, created_at, updated_at FROM algorithms WHERE paper_id = ?1 AND status = ?2 ORDER BY created_at".to_string(),
                        vec![Box::new(paper_id), Box::new(status.clone())],
                    )
                } else {
                    (
                        "SELECT id, paper_id, name, description, steps, inputs, outputs, preconditions, complexity, mathematical_notation, pseudocode, tags, evidence_ids, confidence, status, created_at, updated_at FROM algorithms WHERE paper_id = ?1 ORDER BY created_at".to_string(),
                        vec![Box::new(paper_id)],
                    )
                };

            let mut stmt = conn.prepare(&sql)?;
            let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let algorithms = stmt
                .query_map(refs.as_slice(), |row| {
                    let steps_json: String = row.get(4)?;
                    let inputs_json: String = row.get(5)?;
                    let outputs_json: String = row.get(6)?;
                    let preconditions_json: String = row.get(7)?;
                    let tags_json: String = row.get(11)?;
                    let evidence_ids_json: String = row.get(12)?;
                    let status_str: String = row.get(14)?;

                    Ok(Algorithm {
                        id: row.get(0)?,
                        paper_id: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        steps: serde_json::from_str(&steps_json).unwrap_or_default(),
                        inputs: serde_json::from_str(&inputs_json).unwrap_or_default(),
                        outputs: serde_json::from_str(&outputs_json).unwrap_or_default(),
                        preconditions: serde_json::from_str(&preconditions_json).unwrap_or_default(),
                        complexity: row.get(8)?,
                        mathematical_notation: row.get(9)?,
                        pseudocode: row.get(10)?,
                        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                        evidence_ids: serde_json::from_str(&evidence_ids_json).unwrap_or_default(),
                        confidence: row.get(13)?,
                        status: PatternStatus::from_str(&status_str),
                        created_at: row.get(15)?,
                        updated_at: row.get(16)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(algorithms)
        })
        .await?
    }

    pub async fn update_algorithm_status(&self, id: &str, status: PatternStatus) -> Result<()> {
        let conn = self.conn.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE algorithms SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status.to_string(), now, id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_algorithms_by_paper(&self, paper_id: &str) -> Result<()> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            conn.execute(
                "DELETE FROM algorithms WHERE paper_id = ?1",
                rusqlite::params![paper_id],
            )?;
            Ok(())
        })
        .await?
    }

    /// Search algorithms across all papers with optional filters.
    /// Returns (algorithms_with_paper_title, total_count).
    pub async fn search_algorithms(
        &self,
        query: Option<&str>,
        status: Option<&str>,
        paper_id: Option<&str>,
        tags: Option<&[String]>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<(Algorithm, String)>, usize)> {
        let conn = self.conn.clone();
        let query = query.and_then(|q| {
            if q.trim().is_empty() {
                None
            } else {
                Some(format!("%{}%", q))
            }
        });
        let status = status.map(|s| s.to_string());
        let paper_id = paper_id.map(|s| s.to_string());
        let tags: Option<Vec<String>> = tags.map(|t| t.to_vec());

        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let mut where_clauses = Vec::new();
            let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

            if let Some(ref q) = query {
                let i = bind_values.len() + 1;
                where_clauses.push(format!(
                    "(a.name LIKE ?{} OR a.description LIKE ?{})",
                    i,
                    i + 1
                ));
                bind_values.push(Box::new(q.clone()));
                bind_values.push(Box::new(q.clone()));
            }
            if let Some(ref s) = status {
                where_clauses.push(format!("a.status = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(s.clone()));
            }
            if let Some(ref pid) = paper_id {
                where_clauses.push(format!("a.paper_id = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(pid.clone()));
            }
            if let Some(ref tag_list) = tags {
                for tag in tag_list {
                    where_clauses.push(format!("a.tags LIKE ?{}", bind_values.len() + 1));
                    bind_values.push(Box::new(format!("%\"{}\"%", tag)));
                }
            }

            let where_sql = if where_clauses.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };

            // Count total
            let count_sql = format!(
                "SELECT COUNT(*) FROM algorithms a JOIN papers p ON a.paper_id = p.id {}",
                where_sql
            );
            let total: usize = {
                let mut stmt = conn.prepare(&count_sql)?;
                let refs: Vec<&dyn rusqlite::types::ToSql> =
                    bind_values.iter().map(|b| b.as_ref()).collect();
                stmt.query_row(refs.as_slice(), |row| row.get::<_, i64>(0))? as usize
            };

            // Fetch page
            let query_sql = format!(
                "SELECT a.id, a.paper_id, a.name, a.description, a.steps, a.inputs, a.outputs, \
                 a.preconditions, a.complexity, a.mathematical_notation, a.pseudocode, \
                 a.tags, a.evidence_ids, a.confidence, a.status, a.created_at, a.updated_at, \
                 p.title \
                 FROM algorithms a JOIN papers p ON a.paper_id = p.id \
                 {} ORDER BY a.created_at DESC LIMIT ?{} OFFSET ?{}",
                where_sql,
                bind_values.len() + 1,
                bind_values.len() + 2,
            );
            bind_values.push(Box::new(limit as i64));
            bind_values.push(Box::new(offset as i64));

            let mut stmt = conn.prepare(&query_sql)?;
            let refs: Vec<&dyn rusqlite::types::ToSql> =
                bind_values.iter().map(|b| b.as_ref()).collect();
            let results = stmt
                .query_map(refs.as_slice(), |row| {
                    let steps_json: String = row.get(4)?;
                    let inputs_json: String = row.get(5)?;
                    let outputs_json: String = row.get(6)?;
                    let preconditions_json: String = row.get(7)?;
                    let tags_json: String = row.get(11)?;
                    let evidence_ids_json: String = row.get(12)?;
                    let status_str: String = row.get(14)?;
                    let paper_title: String = row.get(17)?;

                    Ok((
                        Algorithm {
                            id: row.get(0)?,
                            paper_id: row.get(1)?,
                            name: row.get(2)?,
                            description: row.get(3)?,
                            steps: serde_json::from_str(&steps_json).unwrap_or_default(),
                            inputs: serde_json::from_str(&inputs_json).unwrap_or_default(),
                            outputs: serde_json::from_str(&outputs_json).unwrap_or_default(),
                            preconditions: serde_json::from_str(&preconditions_json)
                                .unwrap_or_default(),
                            complexity: row.get(8)?,
                            mathematical_notation: row.get(9)?,
                            pseudocode: row.get(10)?,
                            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                            evidence_ids: serde_json::from_str(&evidence_ids_json)
                                .unwrap_or_default(),
                            confidence: row.get(13)?,
                            status: PatternStatus::from_str(&status_str),
                            created_at: row.get(15)?,
                            updated_at: row.get(16)?,
                        },
                        paper_title,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok((results, total))
        })
        .await?
    }

    pub async fn count_algorithms_by_status(
        &self,
        paper_id: &str,
    ) -> Result<(usize, usize, usize)> {
        let conn = self.conn.clone();
        let paper_id = paper_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = conn
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM algorithms WHERE paper_id = ?1 GROUP BY status",
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
