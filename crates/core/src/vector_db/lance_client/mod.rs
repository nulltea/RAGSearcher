//! LanceDB vector database client
//!
//! NOTE: This file is ~1232 lines (737 implementation + 495 tests).
//! It exceeds the 600-line guideline but is kept as a single coherent unit because:
//! - Tests require access to private methods (must be in same file)
//! - The implementation represents a single logical component (LanceDB client)
//! - Splitting would compromise test coverage and code organization
//!
//! Future refactoring could extract search logic into traits if needed.

use crate::bm25_search::BM25Search;
use crate::types::{ChunkMetadata, SearchResult};
use crate::vector_db::{DatabaseStats, VectorDatabase};
use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
    UInt32Array, types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::TryStreamExt;
use lancedb::Table;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

const BM25_INDEX_PREFIX: &str = "bm25_v2_";

#[derive(Debug, Clone)]
struct StoredSearchRow {
    chunk_id: String,
    file_path: String,
    root_path: Option<String>,
    start_line: usize,
    end_line: usize,
    language: String,
    content: String,
    project: Option<String>,
    page_numbers: Option<Vec<u32>>,
    heading_context: Option<String>,
}

/// LanceDB vector database implementation (embedded, no server required)
/// Includes BM25 hybrid search support using Tantivy with per-project indexes
pub struct LanceVectorDB {
    connection: Connection,
    table_name: String,
    db_path: String,
    /// Per-project BM25 search indexes for keyword matching
    /// Key: hashed root path, Value: BM25Search instance
    bm25_indexes: Arc<RwLock<HashMap<String, BM25Search>>>,
}

impl LanceVectorDB {
    /// Create a new LanceDB instance with default path
    pub async fn new() -> Result<Self> {
        let db_path = Self::default_lancedb_path();
        Self::with_path(&db_path).await
    }

    /// Create a new LanceDB instance with custom path
    pub async fn with_path(db_path: &str) -> Result<Self> {
        tracing::info!("Connecting to LanceDB at: {}", db_path);

        let connection = lancedb::connect(db_path)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        let bm25_indexes = Arc::new(RwLock::new(Self::load_existing_bm25_indexes(db_path)?));

        Ok(Self {
            connection,
            table_name: "code_embeddings".to_string(),
            db_path: db_path.to_string(),
            bm25_indexes,
        })
    }

    /// Get default database path (public for CLI version info)
    pub fn default_lancedb_path() -> String {
        crate::paths::PlatformPaths::default_lancedb_path()
            .to_string_lossy()
            .to_string()
    }

    /// Hash a root path to create a unique identifier for per-project BM25 indexes
    fn hash_root_path(root_path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(root_path.as_bytes());
        let result = hasher.finalize();
        // Use first 16 characters of hex hash for brevity
        format!("{:x}", result)[..16].to_string()
    }

    /// Get the BM25 index path for a specific root path
    fn bm25_path_for_root(&self, root_path: &str) -> String {
        let hash = Self::hash_root_path(root_path);
        format!("{}/{}{}", self.db_path, BM25_INDEX_PREFIX, hash)
    }

    fn load_existing_bm25_indexes(db_path: &str) -> Result<HashMap<String, BM25Search>> {
        let mut indexes = HashMap::new();
        let db_path = Path::new(db_path);

        if !db_path.exists() {
            return Ok(indexes);
        }

        for entry in std::fs::read_dir(db_path).context("Failed to scan BM25 index directory")? {
            let entry = entry.context("Failed to read BM25 index directory entry")?;
            if !entry
                .file_type()
                .context("Failed to inspect BM25 index entry type")?
                .is_dir()
            {
                continue;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(hash) = name.strip_prefix(BM25_INDEX_PREFIX) else {
                continue;
            };

            let index = BM25Search::new(entry.path())
                .with_context(|| format!("Failed to open BM25 index '{}'", name))?;
            indexes.insert(hash.to_string(), index);
        }

        Ok(indexes)
    }

    fn ensure_chunk_ids(metadata: &mut [ChunkMetadata], contents: &[String]) {
        for (index, (meta, content)) in metadata.iter_mut().zip(contents.iter()).enumerate() {
            if meta.chunk_id.is_none() {
                let mut hasher = Sha256::new();
                hasher.update(meta.file_hash.as_bytes());
                hasher.update(b"\0");
                hasher.update(meta.file_path.as_bytes());
                hasher.update(b"\0");
                hasher.update(meta.start_line.to_le_bytes());
                hasher.update(meta.end_line.to_le_bytes());
                hasher.update(b"\0");
                hasher.update(content.as_bytes());
                hasher.update((index as u64).to_le_bytes());
                meta.chunk_id = Some(format!("{:x}", hasher.finalize()));
            }
        }
    }

    fn escape_sql_string(value: &str) -> String {
        value.replace('\'', "''")
    }

    fn build_sql_filter(
        project: Option<&str>,
        root_path: Option<&str>,
        chunk_ids: Option<&[String]>,
    ) -> Option<String> {
        let mut filters = Vec::new();

        if let Some(project) = project {
            filters.push(format!("project = '{}'", Self::escape_sql_string(project)));
        }

        if let Some(root_path) = root_path {
            filters.push(format!(
                "root_path = '{}'",
                Self::escape_sql_string(root_path)
            ));
        }

        if let Some(chunk_ids) = chunk_ids.filter(|ids| !ids.is_empty()) {
            let id_filter = chunk_ids
                .iter()
                .map(|id| format!("id = '{}'", Self::escape_sql_string(id)))
                .collect::<Vec<_>>()
                .join(" OR ");
            filters.push(format!("({})", id_filter));
        }

        (!filters.is_empty()).then(|| filters.join(" AND "))
    }

    fn row_from_batch(batch: &RecordBatch, idx: usize) -> Option<StoredSearchRow> {
        let chunk_id_array = batch
            .column_by_name("id")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let file_path_array = batch
            .column_by_name("file_path")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let root_path_array = batch
            .column_by_name("root_path")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let start_line_array = batch
            .column_by_name("start_line")?
            .as_any()
            .downcast_ref::<UInt32Array>()?;
        let end_line_array = batch
            .column_by_name("end_line")?
            .as_any()
            .downcast_ref::<UInt32Array>()?;
        let language_array = batch
            .column_by_name("language")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let content_array = batch
            .column_by_name("content")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let project_array = batch
            .column_by_name("project")?
            .as_any()
            .downcast_ref::<StringArray>()?;

        let page_numbers = batch
            .column_by_name("page_numbers")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|arr| {
                if arr.is_null(idx) {
                    None
                } else {
                    serde_json::from_str(arr.value(idx)).ok()
                }
            });
        let heading_context = batch
            .column_by_name("heading_context")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|arr| {
                if arr.is_null(idx) {
                    None
                } else {
                    Some(arr.value(idx).to_string())
                }
            });

        Some(StoredSearchRow {
            chunk_id: chunk_id_array.value(idx).to_string(),
            file_path: file_path_array.value(idx).to_string(),
            root_path: if root_path_array.is_null(idx) {
                None
            } else {
                Some(root_path_array.value(idx).to_string())
            },
            start_line: start_line_array.value(idx) as usize,
            end_line: end_line_array.value(idx) as usize,
            language: language_array.value(idx).to_string(),
            content: content_array.value(idx).to_string(),
            project: if project_array.is_null(idx) {
                None
            } else {
                Some(project_array.value(idx).to_string())
            },
            page_numbers,
            heading_context,
        })
    }

    fn row_to_search_result(
        row: StoredSearchRow,
        score: f32,
        combined_score: Option<f32>,
        vector_score: f32,
        keyword_score: Option<f32>,
    ) -> SearchResult {
        SearchResult {
            chunk_id: Some(row.chunk_id),
            file_path: row.file_path,
            root_path: row.root_path,
            content: row.content,
            score,
            combined_score,
            vector_score,
            keyword_score,
            start_line: row.start_line,
            end_line: row.end_line,
            language: row.language,
            project: row.project,
            page_numbers: row.page_numbers,
            heading_context: row.heading_context,
        }
    }

    async fn fetch_rows_by_chunk_ids(
        &self,
        table: &Table,
        chunk_ids: &[String],
        project: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<HashMap<String, StoredSearchRow>> {
        if chunk_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let Some(filter) = Self::build_sql_filter(project, root_path, Some(chunk_ids)) else {
            return Ok(HashMap::new());
        };

        let stream = table
            .query()
            .only_if(filter)
            .select(Select::Columns(vec![
                "id".to_string(),
                "file_path".to_string(),
                "root_path".to_string(),
                "start_line".to_string(),
                "end_line".to_string(),
                "language".to_string(),
                "content".to_string(),
                "project".to_string(),
                "page_numbers".to_string(),
                "heading_context".to_string(),
            ]))
            .execute()
            .await
            .context("Failed to fetch rows by chunk id")?;

        let batches: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .context("Failed to collect rows by chunk id")?;

        let mut rows = HashMap::new();
        for batch in batches {
            for idx in 0..batch.num_rows() {
                if let Some(row) = Self::row_from_batch(&batch, idx) {
                    rows.insert(row.chunk_id.clone(), row);
                }
            }
        }

        Ok(rows)
    }

    fn normalized_query_terms(query_text: &str) -> Vec<String> {
        query_text
            .split(|c: char| !c.is_alphanumeric())
            .map(str::trim)
            .filter(|term| term.len() >= 2)
            .map(|term| term.to_lowercase())
            .collect()
    }

    fn keyword_match_confidence(query_text: &str, content: &str) -> Option<f32> {
        let trimmed_query = query_text.trim();
        if trimmed_query.len() < 2 {
            return None;
        }

        let lowercase_content = content.to_lowercase();
        let lowercase_query = trimmed_query.to_lowercase();

        if lowercase_content.contains(&lowercase_query) {
            return Some(0.92);
        }

        let terms = Self::normalized_query_terms(trimmed_query);
        if !terms.is_empty() && terms.iter().all(|term| lowercase_content.contains(term)) {
            return Some(if terms.len() == 1 { 0.85 } else { 0.78 });
        }

        None
    }

    fn display_score(
        query_text: &str,
        content: &str,
        vector_score: f32,
        keyword_score: Option<f32>,
    ) -> f32 {
        let keyword_confidence = keyword_score
            .and_then(|_| Self::keyword_match_confidence(query_text, content))
            .unwrap_or(0.0);

        vector_score.max(keyword_confidence)
    }

    /// Get or create a BM25 index for a specific root path
    fn get_or_create_bm25(&self, root_path: &str) -> Result<()> {
        let hash = Self::hash_root_path(root_path);

        // Check if already exists (read lock)
        {
            let indexes = self.bm25_indexes.read().map_err(|e| {
                anyhow::anyhow!("Failed to acquire read lock on BM25 indexes: {}", e)
            })?;
            if indexes.contains_key(&hash) {
                return Ok(()); // Already exists
            }
        }

        // Need to create new index (write lock)
        let mut indexes = self
            .bm25_indexes
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock on BM25 indexes: {}", e))?;

        // Double-check after acquiring write lock (another thread might have created it)
        if indexes.contains_key(&hash) {
            return Ok(());
        }

        let bm25_path = self.bm25_path_for_root(root_path);
        tracing::info!(
            "Creating BM25 index for root path '{}' at: {}",
            root_path,
            bm25_path
        );

        let bm25_index = BM25Search::new(&bm25_path)
            .with_context(|| format!("Failed to initialize BM25 index for root: {}", root_path))?;

        indexes.insert(hash, bm25_index);

        Ok(())
    }

    /// Create schema for the embeddings table
    fn create_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("root_path", DataType::Utf8, true),
            Field::new("start_line", DataType::UInt32, false),
            Field::new("end_line", DataType::UInt32, false),
            Field::new("language", DataType::Utf8, false),
            Field::new("extension", DataType::Utf8, false),
            Field::new("file_hash", DataType::Utf8, false),
            Field::new("indexed_at", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("project", DataType::Utf8, true),
            Field::new("page_numbers", DataType::Utf8, true),
            Field::new("heading_context", DataType::Utf8, true),
        ]))
    }

    /// Get or create table. If the table doesn't exist, creates it with a default dimension of 768.
    async fn get_table(&self) -> Result<Table> {
        match self.connection.open_table(&self.table_name).execute().await {
            Ok(table) => Ok(table),
            Err(_) => {
                // Table doesn't exist yet — create it with default dimension
                let schema = Self::create_schema(768);
                let empty_batch = RecordBatch::new_empty(schema.clone());
                let batches =
                    RecordBatchIterator::new(vec![empty_batch].into_iter().map(Ok), schema.clone());
                self.connection
                    .create_table(&self.table_name, Box::new(batches))
                    .execute()
                    .await
                    .context("Failed to create table")?;
                self.connection
                    .open_table(&self.table_name)
                    .execute()
                    .await
                    .context("Failed to open table after creation")
            }
        }
    }

    /// Convert embeddings and metadata to RecordBatch
    fn create_record_batch(
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        schema: Arc<Schema>,
    ) -> Result<RecordBatch> {
        let dimension = embeddings[0].len();

        // Create FixedSizeListArray for vectors
        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            embeddings
                .into_iter()
                .map(|v| Some(v.into_iter().map(Some))),
            dimension as i32,
        );

        // Create arrays for each field
        let id_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.chunk_id.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
        );
        let file_path_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.file_path.as_str())
                .collect::<Vec<_>>(),
        );
        let root_path_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.root_path.as_deref())
                .collect::<Vec<_>>(),
        );
        let start_line_array = UInt32Array::from(
            metadata
                .iter()
                .map(|m| m.start_line as u32)
                .collect::<Vec<_>>(),
        );
        let end_line_array = UInt32Array::from(
            metadata
                .iter()
                .map(|m| m.end_line as u32)
                .collect::<Vec<_>>(),
        );
        let language_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.language.as_deref().unwrap_or("Unknown"))
                .collect::<Vec<_>>(),
        );
        let extension_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.extension.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
        );
        let file_hash_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.file_hash.as_str())
                .collect::<Vec<_>>(),
        );
        let indexed_at_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.indexed_at.to_string())
                .collect::<Vec<_>>(),
        );
        let content_array =
            StringArray::from(contents.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let project_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.project.as_deref())
                .collect::<Vec<_>>(),
        );
        let page_numbers_array = StringArray::from(
            metadata
                .iter()
                .map(|m| {
                    m.page_numbers
                        .as_ref()
                        .map(|pn| serde_json::to_string(pn).unwrap_or_default())
                })
                .collect::<Vec<Option<String>>>(),
        );
        let heading_context_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.heading_context.as_deref())
                .collect::<Vec<_>>(),
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vector_array),
                Arc::new(id_array),
                Arc::new(file_path_array),
                Arc::new(root_path_array),
                Arc::new(start_line_array),
                Arc::new(end_line_array),
                Arc::new(language_array),
                Arc::new(extension_array),
                Arc::new(file_hash_array),
                Arc::new(indexed_at_array),
                Arc::new(content_array),
                Arc::new(project_array),
                Arc::new(page_numbers_array),
                Arc::new(heading_context_array),
            ],
        )
        .context("Failed to create RecordBatch")
    }
}

#[async_trait::async_trait]
impl VectorDatabase for LanceVectorDB {
    async fn initialize(&self, dimension: usize) -> Result<()> {
        tracing::info!(
            "Initializing LanceDB with dimension {} at {}",
            dimension,
            self.db_path
        );

        // Check if table exists
        let table_names = self
            .connection
            .table_names()
            .execute()
            .await
            .context("Failed to list tables")?;

        if table_names.contains(&self.table_name) {
            tracing::info!("Table '{}' already exists", self.table_name);
            return Ok(());
        }

        // Create empty table with schema
        let schema = Self::create_schema(dimension);

        // Create empty RecordBatch
        let empty_batch = RecordBatch::new_empty(schema.clone());

        // Need to wrap in iterator that returns Result<RecordBatch>
        let batches =
            RecordBatchIterator::new(vec![empty_batch].into_iter().map(Ok), schema.clone());

        self.connection
            .create_table(&self.table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create table")?;

        tracing::info!("Created table '{}'", self.table_name);
        Ok(())
    }

    async fn store_embeddings(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        root_path: &str,
    ) -> Result<usize> {
        if embeddings.is_empty() {
            return Ok(0);
        }

        let dimension = embeddings[0].len();
        let schema = Self::create_schema(dimension);
        let mut metadata = metadata;
        Self::ensure_chunk_ids(&mut metadata, &contents);

        let table = self.get_table().await?;

        let batch = Self::create_record_batch(
            embeddings,
            metadata.clone(),
            contents.clone(),
            schema.clone(),
        )?;
        let count = batch.num_rows();

        let batches = RecordBatchIterator::new(vec![batch].into_iter().map(Ok), schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add records to table")?;

        // Ensure BM25 index exists for this root path
        self.get_or_create_bm25(root_path)?;

        // Add documents to per-project BM25 index with file_path for deletion tracking
        let bm25_docs: Vec<_> = (0..count)
            .map(|i| {
                (
                    metadata[i]
                        .chunk_id
                        .clone()
                        .expect("chunk ids assigned before BM25 indexing"),
                    contents[i].clone(),
                    metadata[i].file_path.clone(),
                    metadata[i].project.clone(),
                )
            })
            .collect();

        let hash = Self::hash_root_path(root_path);
        let bm25_indexes = self
            .bm25_indexes
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

        if let Some(bm25) = bm25_indexes.get(&hash) {
            bm25.add_documents(bm25_docs)
                .context("Failed to add documents to BM25 index")?;
        }
        drop(bm25_indexes);

        tracing::info!(
            "Stored {} embeddings with BM25 indexing for root: {}",
            count,
            root_path
        );
        Ok(count)
    }

    async fn search(
        &self,
        query_vector: Vec<f32>,
        query_text: &str,
        limit: usize,
        min_score: f32,
        project: Option<String>,
        root_path: Option<String>,
        hybrid: bool,
    ) -> Result<Vec<SearchResult>> {
        let table = self.get_table().await?;
        let vector_filter = Self::build_sql_filter(project.as_deref(), root_path.as_deref(), None);

        if hybrid {
            let search_limit = limit * 3;
            let query = table
                .vector_search(query_vector)
                .context("Failed to create vector search")?
                .limit(search_limit);

            let stream = if let Some(ref filter) = vector_filter {
                query
                    .only_if(filter.clone())
                    .execute()
                    .await
                    .context("Failed to execute search")?
            } else {
                query.execute().await.context("Failed to execute search")?
            };

            let results: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("Failed to collect search results")?;

            let mut vector_results = Vec::new();
            let mut original_scores: HashMap<String, (f32, Option<f32>)> = HashMap::new();
            let mut rows_by_chunk_id = HashMap::new();

            for batch in &results {
                let distance_array = batch
                    .column_by_name("_distance")
                    .context("Missing _distance column")?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .context("Invalid _distance type")?;
                let chunk_id_array = batch
                    .column_by_name("id")
                    .context("Missing id column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid id type")?;

                for i in 0..batch.num_rows() {
                    let chunk_id = chunk_id_array.value(i).to_string();
                    let distance = distance_array.value(i);
                    let score = 1.0 / (1.0 + distance);
                    vector_results.push((chunk_id.clone(), score));
                    original_scores.insert(chunk_id.clone(), (score, None));

                    if let Some(row) = Self::row_from_batch(batch, i) {
                        rows_by_chunk_id.entry(chunk_id).or_insert(row);
                    }
                }
            }

            if let Some(root_path) = root_path.as_deref() {
                self.get_or_create_bm25(root_path)?;
            }

            let all_bm25_results = {
                let bm25_indexes = self
                    .bm25_indexes
                    .read()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;
                let target_hash = root_path.as_deref().map(Self::hash_root_path);

                let mut all_bm25_results = Vec::new();
                for (root_hash, bm25) in bm25_indexes.iter() {
                    if let Some(ref target_hash) = target_hash
                        && root_hash != target_hash
                    {
                        continue;
                    }

                    tracing::debug!("Searching BM25 index for root hash: {}", root_hash);
                    let results = bm25
                        .search(query_text, search_limit, project.as_deref())
                        .context("Failed to search BM25 index")?;

                    for result in &results {
                        original_scores
                            .entry(result.id.clone())
                            .and_modify(|entry| entry.1 = Some(result.score))
                            .or_insert((0.0, Some(result.score)));
                    }

                    all_bm25_results.extend(results);
                }

                all_bm25_results
            };

            let combined =
                crate::bm25_search::reciprocal_rank_fusion(vector_results, all_bm25_results, limit);
            let missing_chunk_ids: Vec<String> = combined
                .iter()
                .map(|(chunk_id, _)| chunk_id.clone())
                .filter(|chunk_id| !rows_by_chunk_id.contains_key(chunk_id))
                .collect();
            let fetched_rows = self
                .fetch_rows_by_chunk_ids(
                    &table,
                    &missing_chunk_ids,
                    project.as_deref(),
                    root_path.as_deref(),
                )
                .await?;
            rows_by_chunk_id.extend(fetched_rows);

            let mut search_results = Vec::new();
            for (chunk_id, combined_score) in combined {
                let Some(row) = rows_by_chunk_id.remove(&chunk_id) else {
                    tracing::warn!("Could not find result for RRF chunk {}", chunk_id);
                    continue;
                };

                let (vector_score, keyword_score) = original_scores
                    .get(&chunk_id)
                    .cloned()
                    .unwrap_or((0.0, None));
                let display_score =
                    Self::display_score(query_text, &row.content, vector_score, keyword_score);
                let passes_filter = display_score >= min_score;

                if passes_filter {
                    search_results.push(Self::row_to_search_result(
                        row,
                        display_score,
                        Some(combined_score),
                        vector_score,
                        keyword_score,
                    ));
                }
            }

            Ok(search_results)
        } else {
            let query = table
                .vector_search(query_vector)
                .context("Failed to create vector search")?
                .limit(limit);

            let stream = if let Some(ref filter) = vector_filter {
                query
                    .only_if(filter.clone())
                    .execute()
                    .await
                    .context("Failed to execute search")?
            } else {
                query.execute().await.context("Failed to execute search")?
            };

            let results: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("Failed to collect search results")?;

            let mut search_results = Vec::new();
            for batch in results {
                let distance_array = batch
                    .column_by_name("_distance")
                    .context("Missing _distance column")?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .context("Invalid _distance type")?;

                for i in 0..batch.num_rows() {
                    let distance = distance_array.value(i);
                    let score = 1.0 / (1.0 + distance);

                    if score >= min_score
                        && let Some(row) = Self::row_from_batch(&batch, i)
                    {
                        search_results.push(Self::row_to_search_result(
                            row,
                            score,
                            None,
                            score,
                            None,
                        ));
                    }
                }
            }

            Ok(search_results)
        }
    }

    async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        // Delete from BM25 index first (using file_path field)
        // Delete from all per-project BM25 indexes
        // Must be done in a scope to drop lock before await
        {
            let bm25_indexes = self
                .bm25_indexes
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

            for (root_hash, bm25) in bm25_indexes.iter() {
                bm25.delete_by_file_path(file_path)
                    .context("Failed to delete from BM25 index")?;
                tracing::debug!(
                    "Deleted BM25 entries for file: {} in index: {}",
                    file_path,
                    root_hash
                );
            }
        } // bm25_indexes dropped here

        let table = self.get_table().await?;

        // LanceDB uses SQL-like delete
        let filter = format!("file_path = '{}'", file_path);

        table
            .delete(&filter)
            .await
            .context("Failed to delete records")?;

        tracing::info!("Deleted embeddings for file: {}", file_path);

        // LanceDB doesn't return count directly, return 0 as placeholder
        Ok(0)
    }

    async fn delete_by_project(&self, project: &str) -> Result<usize> {
        let table = self.get_table().await?;

        let filter = format!("project = '{}'", project);
        table
            .delete(&filter)
            .await
            .context("Failed to delete records by project")?;

        tracing::info!("Deleted embeddings for project: {}", project);
        Ok(0)
    }

    async fn clear(&self) -> Result<()> {
        // Drop and recreate table (empty namespace array for default namespace)
        self.connection
            .drop_table(&self.table_name, &[])
            .await
            .context("Failed to drop table")?;

        // Clear all per-project BM25 indexes
        let bm25_indexes = self
            .bm25_indexes
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

        for (root_hash, bm25) in bm25_indexes.iter() {
            bm25.clear().context("Failed to clear BM25 index")?;
            tracing::info!("Cleared BM25 index for root hash: {}", root_hash);
        }
        drop(bm25_indexes);

        tracing::info!("Cleared all embeddings and all per-project BM25 indexes");
        Ok(())
    }

    async fn get_statistics(&self) -> Result<DatabaseStats> {
        let table = self.get_table().await?;

        // Count total vectors
        let count_result = table
            .count_rows(None)
            .await
            .context("Failed to count rows")?;

        // Get language breakdown by scanning the table
        let stream = table
            .query()
            .select(lancedb::query::Select::Columns(vec![
                "language".to_string(),
            ]))
            .execute()
            .await
            .context("Failed to query languages")?;

        let query_result: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .context("Failed to collect language data")?;

        let mut language_counts: HashMap<String, usize> = HashMap::new();

        for batch in query_result {
            let language_array = batch
                .column_by_name("language")
                .context("Missing language column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid language type")?;

            for i in 0..batch.num_rows() {
                let language = language_array.value(i);
                *language_counts.entry(language.to_string()).or_insert(0) += 1;
            }
        }

        let mut language_breakdown: Vec<(String, usize)> = language_counts.into_iter().collect();
        language_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(DatabaseStats {
            total_points: count_result,
            total_vectors: count_result,
            language_breakdown,
        })
    }

    async fn flush(&self) -> Result<()> {
        // LanceDB persists automatically, no explicit flush needed
        Ok(())
    }

    async fn count_by_root_path(&self, root_path: &str) -> Result<usize> {
        let table = self.get_table().await?;

        // Use SQL-like filter to count rows with matching root_path
        let filter = format!("root_path = '{}'", root_path);
        let count = table
            .count_rows(Some(filter))
            .await
            .context("Failed to count rows by root path")?;

        Ok(count)
    }

    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>> {
        let table = self.get_table().await?;

        // Query file_path column filtered by root_path
        let filter = format!("root_path = '{}'", root_path);
        let stream = table
            .query()
            .only_if(filter)
            .select(lancedb::query::Select::Columns(vec![
                "file_path".to_string(),
            ]))
            .execute()
            .await
            .context("Failed to query indexed files")?;

        let results: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .context("Failed to collect file paths")?;

        // Extract unique file paths
        let mut file_paths = std::collections::HashSet::new();

        for batch in results {
            let file_path_array = batch
                .column_by_name("file_path")
                .context("Missing file_path column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid file_path type")?;

            for i in 0..batch.num_rows() {
                file_paths.insert(file_path_array.value(i).to_string());
            }
        }

        Ok(file_paths.into_iter().collect())
    }
}

#[cfg(test)]
mod tests;
